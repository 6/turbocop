use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/MultipleComparison: Avoid comparing a variable with multiple items
/// in a conditional, use `Array#include?` instead.
///
/// Corpus investigation (round 2): 16 FPs, 32 FNs.
///
/// FP root cause: The cop flagged comparisons where the "value" side was a
/// local variable (e.g., `exit_status == 0 || exit_status == still_active`).
/// RuboCop treats `lvar == lvar` as a `simple_double_comparison` and skips
/// it entirely — it only counts comparisons where the value is NOT an lvar.
///
/// FN root cause 1: The `inside_or` flag was set globally when entering a
/// root OrNode, which prevented detection of independent OrNode groups
/// nested inside `&&` expressions (e.g., `(rotation == 0 || rotation == 180)`
/// inside a larger `&& ||` chain).
///
/// FN root cause 2: The variable/value identification was reversed for cases
/// like `it[:from][:x] == outer_left_x`. The call node should be the
/// "variable" and the lvar should be the "value", matching RuboCop's
/// `simple_comparison_lhs/rhs` patterns: `(send {lvar call} :== $_)`.
///
/// Fixes:
/// - Skip `lvar == lvar` comparisons (simple_double_comparison).
/// - Match RuboCop's variable/value identification: `{lvar, call}` is the
///   variable, everything else is the value. AllowMethodComparison only
///   applies when the VALUE is a call.
/// - After processing a root OrNode, manually flatten its || chain and
///   visit non-Or leaf children for independent nested OrNodes, instead of
///   using `inside_or` flag which incorrectly blocked OrNodes inside `&&`.
///
/// Corpus investigation (round 3): mixed all-`==` chains still produced false
/// negatives when the repeated variable group was followed by a different
/// comparison later in the same `||` tree, for example:
/// `a == x || a == y || b == z` or `x == 0 || x == 24 || y == 0 || y == 13`.
/// RuboCop keeps the first repeated variable group and stops collecting once a
/// different variable appears; the old implementation merged both sides
/// symmetrically and dropped the whole chain. The fix scans the `||` leaves
/// left-to-right, preserving the first repeated group while still ignoring
/// later subchains that RuboCop does not flag.
///
/// Corpus investigation (round 4): an allowed method-value comparison at the
/// front of an all-`==` chain could still poison the scan state and hide a
/// later repeated local-variable group, for example
/// `l == @buffer.current_line || e == :space || e == :comment`. RuboCop treats
/// the first comparison as comparison-shaped for `nested_comparison?`, but it
/// does not add that variable to `find_offending_var`. The fix keeps these
/// nodes in the tree shape while skipping them entirely during offending-var
/// grouping, so later repeated comparisons are still detected.
pub struct MultipleComparison;

/// Result of analyzing a single `==` comparison.
enum ComparisonResult {
    /// A valid comparison that contributes to offending-var grouping.
    Counted { var_src: Vec<u8> },
    /// An allowed method-value comparison that still counts as comparison-shaped
    /// for `nested_comparison?`, but should not affect the offending variable.
    SkippedMethodValue,
    /// Both sides are local variables — skip but don't break chain.
    DoubleVar,
}

impl MultipleComparison {
    /// Extract a single `==` comparison from a node, if it matches RuboCop's
    /// `simple_comparison` / `simple_double_comparison?` logic.
    fn comparison_result<'a>(
        node: &'a ruby_prism::Node<'a>,
        allow_method: bool,
    ) -> Option<ComparisonResult> {
        let call = node.as_call_node()?;
        if call.name().as_slice() != b"==" {
            return None;
        }

        let lhs = call.receiver()?;
        let rhs_args = call.arguments()?;
        let rhs_list: Vec<_> = rhs_args.arguments().iter().collect();
        if rhs_list.len() != 1 {
            return None;
        }

        Self::classify_comparison(&lhs, &rhs_list[0], allow_method)
    }

    /// Scan an all-`==` `||` chain left-to-right, matching RuboCop's
    /// `find_offending_var` behavior: keep counting the first repeated variable
    /// group and stop as soon as a different compared variable appears.
    fn scan_comparison_chain<'a>(
        node: &'a ruby_prism::Node<'a>,
        allow_method: bool,
        first_var: &mut Option<Vec<u8>>,
        count: &mut usize,
        blocked: &mut bool,
    ) {
        if *blocked {
            return;
        }

        if let Some(or_node) = node.as_or_node() {
            let lhs = or_node.left();
            let rhs = or_node.right();
            Self::scan_comparison_chain(&lhs, allow_method, first_var, count, blocked);
            Self::scan_comparison_chain(&rhs, allow_method, first_var, count, blocked);
            return;
        }

        let Some(result) = Self::comparison_result(node, allow_method) else {
            return;
        };

        match result {
            ComparisonResult::Counted { var_src } => match first_var {
                Some(existing) if existing == &var_src => {
                    *count += 1;
                }
                Some(_) => {
                    *blocked = true;
                }
                None => {
                    *first_var = Some(var_src);
                    *count += 1;
                }
            },
            ComparisonResult::SkippedMethodValue => {
                // AllowMethodComparison=true: keep the node comparison-shaped
                // without letting it define the offending variable.
            }
            ComparisonResult::DoubleVar => {
                // `lvar == lvar` participates in the tree shape but is ignored.
            }
        }
    }

    /// Classify a `==` comparison, matching RuboCop's `simple_comparison_lhs/rhs`
    /// and `simple_double_comparison?` patterns.
    ///
    /// RuboCop patterns:
    /// - `simple_double_comparison?`: `(send lvar :== lvar)` → skip
    /// - `simple_comparison_lhs`: `(send {lvar call} :== $_)` → var=lhs, value=rhs
    /// - `simple_comparison_rhs`: `(send $_ :== {lvar call})` → var=rhs, value=lhs
    fn classify_comparison<'a>(
        lhs: &'a ruby_prism::Node<'a>,
        rhs: &'a ruby_prism::Node<'a>,
        allow_method: bool,
    ) -> Option<ComparisonResult> {
        let lhs_is_lvar = lhs.as_local_variable_read_node().is_some();
        let rhs_is_lvar = rhs.as_local_variable_read_node().is_some();
        let lhs_is_call = lhs.as_call_node().is_some();
        let rhs_is_call = rhs.as_call_node().is_some();

        // simple_double_comparison: both sides are lvars
        if lhs_is_lvar && rhs_is_lvar {
            return Some(ComparisonResult::DoubleVar);
        }

        // Try simple_comparison_lhs: (send {lvar call} :== $_)
        // The variable is the {lvar, call} side, value is the other side
        if lhs_is_lvar || lhs_is_call {
            let var_src = lhs.location().as_slice().to_vec();
            let value_is_call = rhs_is_call;

            // When AllowMethodComparison is false and variable is a call, RuboCop skips
            if lhs_is_call && !allow_method {
                return None;
            }

            if allow_method && value_is_call {
                return Some(ComparisonResult::SkippedMethodValue);
            }
            return Some(ComparisonResult::Counted { var_src });
        }

        // Try simple_comparison_rhs: (send $_ :== {lvar call})
        if rhs_is_lvar || rhs_is_call {
            let var_src = rhs.location().as_slice().to_vec();
            let value_is_call = lhs_is_call;

            if rhs_is_call && !allow_method {
                return None;
            }

            if allow_method && value_is_call {
                return Some(ComparisonResult::SkippedMethodValue);
            }
            return Some(ComparisonResult::Counted { var_src });
        }

        // Neither side is an lvar or call — not a matchable comparison
        None
    }

    /// Recursively visit non-OrNode leaf nodes from an || chain.
    /// This flattens the chain and visits each leaf with the given visitor.
    fn visit_or_leaves<'a>(
        node: &ruby_prism::Node<'a>,
        visitor: &mut MultipleComparisonVisitor<'a>,
    ) {
        if let Some(or_node) = node.as_or_node() {
            let lhs = or_node.left();
            let rhs = or_node.right();
            Self::visit_or_leaves(&lhs, visitor);
            Self::visit_or_leaves(&rhs, visitor);
        } else {
            visitor.visit(node);
        }
    }
}

impl Cop for MultipleComparison {
    fn name(&self) -> &'static str {
        "Style/MultipleComparison"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_method = config.get_bool("AllowMethodComparison", true);
        let threshold = config.get_usize("ComparisonsThreshold", 2);

        let mut visitor = MultipleComparisonVisitor {
            cop: self,
            source,
            allow_method,
            threshold,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MultipleComparisonVisitor<'a> {
    cop: &'a MultipleComparison,
    source: &'a SourceFile,
    allow_method: bool,
    threshold: usize,
    diagnostics: Vec<Diagnostic>,
}

impl MultipleComparisonVisitor<'_> {
    /// Check whether a node is a RuboCop-style nested comparison tree.
    fn is_comparison<'a>(&self, node: &'a ruby_prism::Node<'a>) -> bool {
        if let Some(or_node) = node.as_or_node() {
            let lhs = or_node.left();
            let rhs = or_node.right();
            self.is_comparison(&lhs) && self.is_comparison(&rhs)
        } else {
            MultipleComparison::comparison_result(node, self.allow_method).is_some()
        }
    }
}

impl<'a> Visit<'a> for MultipleComparisonVisitor<'a> {
    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'a>) {
        let lhs = node.left();
        let rhs = node.right();

        // Process only all-comparison || chains. This intentionally returns
        // early even when the chain is not an offense, so later subchains in
        // the same root || expression are not flagged independently.
        if self.is_comparison(&lhs) && self.is_comparison(&rhs) {
            let mut first_var = None;
            let mut count = 0;
            let mut blocked = false;
            MultipleComparison::scan_comparison_chain(
                &lhs,
                self.allow_method,
                &mut first_var,
                &mut count,
                &mut blocked,
            );
            MultipleComparison::scan_comparison_chain(
                &rhs,
                self.allow_method,
                &mut first_var,
                &mut count,
                &mut blocked,
            );

            if first_var.is_some() && count >= self.threshold {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.".to_string(),
                ));
            }

            // Don't recurse: all leaves are == comparisons with no nested OrNodes.
            return;
        }

        // This OrNode chain contains non-== branches (mixed chain).
        // Don't flag it, but recurse into children to find independent OrNode groups.
        MultipleComparison::visit_or_leaves(&lhs, self);
        MultipleComparison::visit_or_leaves(&rhs, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleComparison, "cops/style/multiple_comparison");
}
