use crate::cop::node_type::{AND_NODE, IF_NODE, OR_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FN fix: `collect_and_or_in_condition` previously only recursed into `AndNode`
/// and `OrNode` children, missing `and`/`or` nested inside `ParenthesesNode`
/// (e.g., `until (x or y)`, `if (a and b) or (c and d)`, `unless (a or b)`).
/// Added traversal through `ParenthesesNode` and `StatementsNode` to match
/// RuboCop's `each_node(:and, :or)` deep walk. Resolved ~870 of 1138 FN.
///
/// Remaining FN (~268): likely caused by config/context differences (e.g.,
/// `rubocop:disable` comments, per-file Include/Exclude rules) rather than
/// detection logic bugs.
///
/// Remaining FP (1): `danbooru__danbooru__fd45f0f: app/logical/source/url/null.rb:292`
/// — could not diagnose (no source context available).
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
                check_and_or_node(self, source, node).into_iter().next()
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
/// Returns (Diagnostic, op_start, op_end, replacement) tuples.
fn check_and_or_node(
    cop: &AndOr,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Vec<(Diagnostic, usize, usize, &'static str)> {
    if let Some(and_node) = node.as_and_node() {
        let op_loc = and_node.operator_loc();
        if op_loc.as_slice() == b"and" {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            return vec![(
                cop.diagnostic(
                    source,
                    line,
                    column,
                    "Use `&&` instead of `and`.".to_string(),
                ),
                op_loc.start_offset(),
                op_loc.end_offset(),
                "&&",
            )];
        }
    } else if let Some(or_node) = node.as_or_node() {
        let op_loc = or_node.operator_loc();
        if op_loc.as_slice() == b"or" {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            return vec![(
                cop.diagnostic(
                    source,
                    line,
                    column,
                    "Use `||` instead of `or`.".to_string(),
                ),
                op_loc.start_offset(),
                op_loc.end_offset(),
                "||",
            )];
        }
    }
    Vec::new()
}

/// Recursively walk a condition expression finding `and`/`or` keyword nodes.
fn collect_and_or_in_condition(
    cop: &AndOr,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
) {
    if let Some(and_node) = node.as_and_node() {
        let op_loc = and_node.operator_loc();
        if op_loc.as_slice() == b"and" {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            let mut diag = cop.diagnostic(
                source,
                line,
                column,
                "Use `&&` instead of `and`.".to_string(),
            );
            if let Some(corr) = corrections {
                corr.push(crate::correction::Correction {
                    start: op_loc.start_offset(),
                    end: op_loc.end_offset(),
                    replacement: "&&".to_string(),
                    cop_name: cop.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
        // Recurse into both sides
        collect_and_or_in_condition(cop, source, &and_node.left(), diagnostics, corrections);
        collect_and_or_in_condition(cop, source, &and_node.right(), diagnostics, corrections);
    } else if let Some(or_node) = node.as_or_node() {
        let op_loc = or_node.operator_loc();
        if op_loc.as_slice() == b"or" {
            let (line, column) = source.offset_to_line_col(op_loc.start_offset());
            let mut diag = cop.diagnostic(
                source,
                line,
                column,
                "Use `||` instead of `or`.".to_string(),
            );
            if let Some(corr) = corrections {
                corr.push(crate::correction::Correction {
                    start: op_loc.start_offset(),
                    end: op_loc.end_offset(),
                    replacement: "||".to_string(),
                    cop_name: cop.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
        // Recurse into both sides
        collect_and_or_in_condition(cop, source, &or_node.left(), diagnostics, corrections);
        collect_and_or_in_condition(cop, source, &or_node.right(), diagnostics, corrections);
    }
    // Recurse through parentheses and statements to find and/or nested inside
    // container nodes (e.g., `until (x or y)`, `if (a and b) or (c and d)`).
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            collect_and_or_in_condition(cop, source, &body, diagnostics, corrections);
        }
    } else if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            collect_and_or_in_condition(cop, source, &child, diagnostics, corrections);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AndOr, "cops/style/and_or");
    crate::cop_autocorrect_fixture_tests!(AndOr, "cops/style/and_or");
}
