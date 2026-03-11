use crate::cop::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Collect local variable names assigned in a condition node (recursively).
/// Handles `AndNode`, `OrNode`, `ParenthesesNode`, and direct `LocalVariableWriteNode`.
fn collect_assigned_variables(node: &ruby_prism::Node<'_>, names: &mut Vec<Vec<u8>>) {
    if let Some(write) = node.as_local_variable_write_node() {
        names.push(write.name().as_slice().to_vec());
    } else if let Some(and_node) = node.as_and_node() {
        collect_assigned_variables(&and_node.left(), names);
        collect_assigned_variables(&and_node.right(), names);
    } else if let Some(or_node) = node.as_or_node() {
        collect_assigned_variables(&or_node.left(), names);
        collect_assigned_variables(&or_node.right(), names);
    } else if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            collect_assigned_variables(&body, names);
        }
    } else if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            collect_assigned_variables(&stmt, names);
        }
    }
}

/// Check if the inner branch's condition references a variable assigned in the outer condition.
/// Mirrors RuboCop's `use_variable_assignment_in_condition?`.
fn has_variable_assignment_dependency(
    outer_condition: &ruby_prism::Node<'_>,
    inner_branch: &ruby_prism::Node<'_>,
) -> bool {
    let mut assigned = Vec::new();
    collect_assigned_variables(outer_condition, &mut assigned);
    if assigned.is_empty() {
        return false;
    }

    // Only applies when inner branch is an if node (not unless), matching RuboCop
    let inner_if = match inner_branch.as_if_node() {
        Some(if_node) => if_node,
        None => return false,
    };

    // RuboCop checks if the inner condition's source text matches an assigned variable name.
    // We check if the inner condition is a LocalVariableReadNode with a matching name.
    let inner_cond = inner_if.predicate();
    if let Some(read) = inner_cond.as_local_variable_read_node() {
        let read_name = read.name().as_slice();
        return assigned.iter().any(|n| n.as_slice() == read_name);
    }

    false
}

/// Corpus investigation (2026-03-10): 3 FP, 0 FN.
/// Root cause: missing `use_variable_assignment_in_condition?` check from RuboCop.
/// RuboCop skips the offense when the outer condition assigns a variable that is
/// referenced in the inner condition (e.g., `if var = foo; do_something if var; end`).
/// Fix: added `has_variable_assignment_dependency()` to check for local variable
/// writes in the outer condition whose names appear as reads in the inner condition.
pub struct SoleNestedConditional;

impl Cop for SoleNestedConditional {
    fn name(&self) -> &'static str {
        "Style/SoleNestedConditional"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_modifier = config.get_bool("AllowModifier", false);

        // Check if this is an if/unless without else
        let (kw_loc, statements, has_else, outer_condition) =
            if let Some(if_node) = node.as_if_node() {
                let kw = match if_node.if_keyword_loc() {
                    Some(loc) => loc,
                    None => return, // ternary
                };
                if kw.as_slice() == b"elsif" {
                    return;
                }
                (
                    kw,
                    if_node.statements(),
                    if_node.subsequent().is_some(),
                    Some(if_node.predicate()),
                )
            } else if let Some(unless_node) = node.as_unless_node() {
                (
                    unless_node.keyword_loc(),
                    unless_node.statements(),
                    unless_node.else_clause().is_some(),
                    Some(unless_node.predicate()),
                )
            } else {
                return;
            };

        if has_else {
            return;
        }

        let stmts = match statements {
            Some(s) => s,
            None => return,
        };

        let body: Vec<_> = stmts.body().iter().collect();
        if body.len() != 1 {
            return;
        }

        // Skip when outer condition assigns a variable used in the inner condition
        if let Some(ref cond) = outer_condition {
            if has_variable_assignment_dependency(cond, &body[0]) {
                return;
            }
        }

        // Check if the sole statement is another if/unless without else
        let is_nested_if = if let Some(inner_if) = body[0].as_if_node() {
            let inner_kw = match inner_if.if_keyword_loc() {
                Some(loc) => loc,
                None => return, // ternary
            };

            if allow_modifier {
                // Skip if inner is modifier form
                if inner_if.end_keyword_loc().is_none() {
                    return;
                }
            }

            // Inner if must not have else
            if inner_if.subsequent().is_some() {
                return;
            }

            inner_kw.as_slice() == b"if"
        } else if let Some(inner_unless) = body[0].as_unless_node() {
            if allow_modifier && inner_unless.end_keyword_loc().is_none() {
                return;
            }

            if inner_unless.else_clause().is_some() {
                return;
            }

            true
        } else {
            false
        };

        if !is_nested_if {
            return;
        }

        // RuboCop reports the offense on the inner conditional's keyword, not the outer
        let inner_kw_loc = if let Some(inner_if) = body[0].as_if_node() {
            inner_if.if_keyword_loc().unwrap_or(kw_loc)
        } else if let Some(inner_unless) = body[0].as_unless_node() {
            inner_unless.keyword_loc()
        } else {
            kw_loc
        };

        let (line, column) = source.offset_to_line_col(inner_kw_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Consider merging nested conditions into outer `if` conditions.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SoleNestedConditional, "cops/style/sole_nested_conditional");
}
