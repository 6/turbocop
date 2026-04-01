use crate::cop::node_type::{AND_NODE, IF_NODE, OR_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Detects `and`/`or` keyword operators inside conditional predicates.
///
/// The original implementation only recursed through a few wrapper nodes, which
/// missed operators nested under `not`, method-call receivers and arguments,
/// index arguments, and block bodies. It now walks the full predicate subtree
/// with a Prism visitor and reports keyword operators in source order.
///
/// Prism also represents `case/in` guards as `IfNode`/`UnlessNode` inside the
/// pattern. RuboCop does not treat those guards as regular conditionals for
/// `Style/AndOr`, so they are skipped to avoid false positives like
/// `in pattern if a and b`.
pub struct AndOr;

impl Cop for AndOr {
    fn name(&self) -> &'static str {
        "Style/AndOr"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            AND_NODE,
            IF_NODE,
            OR_NODE,
            UNLESS_NODE,
            UNTIL_NODE,
            WHILE_NODE,
        ]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "conditionals");

        if enforced_style == "always" {
            // In "always" mode, flag every `and` and `or` keyword
            if let Some((diag, op_start, op_end, replacement)) =
                check_and_or_node(self, source, node)
            {
                let mut d = diag;
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: op_start,
                        end: op_end,
                        replacement: replacement.to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    d.corrected = true;
                }
                diagnostics.push(d);
            }
            return;
        }

        // "conditionals" mode: only flag `and`/`or` inside conditions of if/while/until
        if is_pattern_matching_guard(source, node) {
            return;
        }

        let condition = if let Some(if_node) = node.as_if_node() {
            if_node.predicate()
        } else if let Some(unless_node) = node.as_unless_node() {
            unless_node.predicate()
        } else if let Some(while_node) = node.as_while_node() {
            while_node.predicate()
        } else if let Some(until_node) = node.as_until_node() {
            until_node.predicate()
        } else {
            return;
        };

        // Walk the condition tree for and/or nodes
        collect_and_or_in_condition(self, source, &condition, diagnostics, &mut corrections);
    }
}

/// Check if a single node is an `and`/`or` keyword and report it.
/// Returns (Diagnostic, op_start, op_end, replacement) if the node is a keyword operator.
fn check_and_or_node(
    cop: &AndOr,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Option<(Diagnostic, usize, usize, &'static str)> {
    if let Some(and_node) = node.as_and_node() {
        let op_loc = and_node.operator_loc();
        if op_loc.as_slice() == b"and" {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            return Some((
                cop.diagnostic(
                    source,
                    line,
                    column,
                    "Use `&&` instead of `and`.".to_string(),
                ),
                op_loc.start_offset(),
                op_loc.end_offset(),
                "&&",
            ));
        }
    } else if let Some(or_node) = node.as_or_node() {
        let op_loc = or_node.operator_loc();
        if op_loc.as_slice() == b"or" {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            return Some((
                cop.diagnostic(
                    source,
                    line,
                    column,
                    "Use `||` instead of `or`.".to_string(),
                ),
                op_loc.start_offset(),
                op_loc.end_offset(),
                "||",
            ));
        }
    }
    None
}

fn emit_and_or_offense(
    cop: &AndOr,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
) {
    if let Some((mut diag, op_start, op_end, replacement)) = check_and_or_node(cop, source, node) {
        if let Some(corr) = corrections {
            corr.push(crate::correction::Correction {
                start: op_start,
                end: op_end,
                replacement: replacement.to_string(),
                cop_name: cop.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[derive(Default)]
struct ConditionOperatorCollector<'pr> {
    operators: Vec<ruby_prism::Node<'pr>>,
}

impl<'pr> Visit<'pr> for ConditionOperatorCollector<'pr> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        if node.as_and_node().is_some() || node.as_or_node().is_some() {
            self.operators.push(node);
        }
    }
}

fn operator_start_offset(node: &ruby_prism::Node<'_>) -> usize {
    if let Some(and_node) = node.as_and_node() {
        return and_node.operator_loc().start_offset();
    }
    if let Some(or_node) = node.as_or_node() {
        return or_node.operator_loc().start_offset();
    }
    usize::MAX
}

/// Check if an `IfNode`/`UnlessNode` is the guard attached to a `case/in` pattern.
///
/// Prism models `in pattern if guard` as an `IfNode` whose source range starts at
/// the pattern body, so the text from the start of the line up to the node is just `in`.
fn is_pattern_matching_guard(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    if node.as_if_node().is_none() && node.as_unless_node().is_none() {
        return false;
    }

    let loc = node.location();
    let start = loc.start_offset();
    let (line, _) = source.offset_to_line_col(start);
    let Some(line_start) = source.line_col_to_offset(line, 0) else {
        return false;
    };
    let Some(prefix) = source.try_byte_slice(line_start, start) else {
        return false;
    };

    prefix.trim() == "in"
}

/// Deep-walk a predicate subtree to find `and`/`or` keyword operators anywhere
/// inside it, matching RuboCop's `each_node(:and, :or)` behavior.
fn collect_and_or_in_condition(
    cop: &AndOr,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
) {
    let mut collector = ConditionOperatorCollector::default();
    collector.visit(node);
    collector.operators.sort_by_key(operator_start_offset);

    for operator in collector.operators {
        emit_and_or_offense(cop, source, &operator, diagnostics, corrections);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AndOr, "cops/style/and_or");
    crate::cop_autocorrect_fixture_tests!(AndOr, "cops/style/and_or");
}
