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
pub struct MultipleComparison;

/// Result of analyzing a single `==` comparison.
enum ComparisonResult {
    /// A valid comparison: variable source bytes and whether it counts.
    /// count=0 means skipped (e.g., AllowMethodComparison), count=1 means counted.
    Valid { var_src: Vec<u8>, count: usize },
    /// Both sides are local variables — skip but don't break chain.
    DoubleVar,
}

impl MultipleComparison {
    /// Recursively collect == comparisons joined by ||, returning the variable
    /// being compared if consistent, along with the comparison count.
    /// Matches RuboCop's `find_offending_var` logic.
    fn collect_comparisons<'a>(
        node: &'a ruby_prism::Node<'a>,
        allow_method: bool,
    ) -> Option<(Vec<u8>, usize)> {
        // Handle OrNode: a == x || a == y
        if let Some(or_node) = node.as_or_node() {
            let lhs = or_node.left();
            let rhs = or_node.right();

            let lhs_result = Self::collect_comparisons(&lhs, allow_method);
            let rhs_result = Self::collect_comparisons(&rhs, allow_method);

            match (lhs_result, rhs_result) {
                (Some((lhs_var, lhs_count)), Some((rhs_var, rhs_count))) => {
                    if lhs_var == rhs_var {
                        return Some((lhs_var, lhs_count + rhs_count));
                    }
                    // Different variables but might share if one is empty (skipped method comparison)
                    if lhs_count == 0 {
                        return Some((rhs_var, rhs_count));
                    }
                    if rhs_count == 0 {
                        return Some((lhs_var, lhs_count));
                    }
                    return None;
                }
                (Some(_), None) | (None, Some(_)) => {
                    return None;
                }
                (None, None) => return None,
            }
        }

        // Handle CallNode with ==
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"==" {
                let lhs = call.receiver()?;
                let rhs_args = call.arguments()?;
                let rhs_list: Vec<_> = rhs_args.arguments().iter().collect();
                if rhs_list.len() != 1 {
                    return None;
                }
                let rhs = &rhs_list[0];

                let result = Self::classify_comparison(&lhs, rhs, allow_method)?;
                return match result {
                    ComparisonResult::Valid { var_src, count } => Some((var_src, count)),
                    // simple_double_comparison: both sides are lvars — skip but keep chain alive
                    // Return a dummy variable with count 0 so the chain continues
                    ComparisonResult::DoubleVar => Some((lhs.location().as_slice().to_vec(), 0)),
                };
            }
        }
        None
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
                return Some(ComparisonResult::Valid { var_src, count: 0 });
            }
            return Some(ComparisonResult::Valid { var_src, count: 1 });
        }

        // Try simple_comparison_rhs: (send $_ :== {lvar call})
        if rhs_is_lvar || rhs_is_call {
            let var_src = rhs.location().as_slice().to_vec();
            let value_is_call = lhs_is_call;

            if rhs_is_call && !allow_method {
                return None;
            }

            if allow_method && value_is_call {
                return Some(ComparisonResult::Valid { var_src, count: 0 });
            }
            return Some(ComparisonResult::Valid { var_src, count: 1 });
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
    /// Check if the lhs and rhs of an OrNode form a chain of only `==` comparisons.
    /// Matches RuboCop's `nested_comparison?` check.
    fn nested_comparison_or<'a>(
        lhs: &'a ruby_prism::Node<'a>,
        rhs: &'a ruby_prism::Node<'a>,
    ) -> bool {
        Self::is_comparison(lhs) && Self::is_comparison(rhs)
    }

    fn is_comparison<'a>(node: &'a ruby_prism::Node<'a>) -> bool {
        if let Some(or_node) = node.as_or_node() {
            let lhs = or_node.left();
            let rhs = or_node.right();
            Self::is_comparison(&lhs) && Self::is_comparison(&rhs)
        } else if let Some(call) = node.as_call_node() {
            call.name().as_slice() == b"=="
        } else {
            false
        }
    }
}

impl<'a> Visit<'a> for MultipleComparisonVisitor<'a> {
    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'a>) {
        let lhs = node.left();
        let rhs = node.right();

        // Check if this is an || chain consisting entirely of == comparisons.
        if Self::nested_comparison_or(&lhs, &rhs) {
            // Process the full chain
            let lhs_result = MultipleComparison::collect_comparisons(&lhs, self.allow_method);
            let rhs_result = MultipleComparison::collect_comparisons(&rhs, self.allow_method);

            let result = match (lhs_result, rhs_result) {
                (Some((lhs_var, lhs_count)), Some((rhs_var, rhs_count))) => {
                    if lhs_var == rhs_var {
                        Some(lhs_count + rhs_count)
                    } else if lhs_count == 0 {
                        Some(rhs_count)
                    } else if rhs_count == 0 {
                        Some(lhs_count)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(count) = result {
                if count >= self.threshold {
                    let loc = node.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Avoid comparing a variable with multiple items in a conditional, use `Array#include?` instead.".to_string(),
                    ));
                }
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
