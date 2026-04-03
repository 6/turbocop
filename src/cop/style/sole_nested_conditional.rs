use crate::cop::shared::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

fn collect_assigned_target_variables(node: &ruby_prism::Node<'_>, names: &mut Vec<Vec<u8>>) {
    if let Some(target) = node.as_local_variable_target_node() {
        names.push(target.name().as_slice().to_vec());
    } else if let Some(targets) = node.as_multi_target_node() {
        for target in targets.lefts().iter() {
            collect_assigned_target_variables(&target, names);
        }
        if let Some(rest) = targets.rest() {
            if let Some(splat) = rest.as_splat_node() {
                if let Some(expr) = splat.expression() {
                    collect_assigned_target_variables(&expr, names);
                }
            }
        }
        for target in targets.rights().iter() {
            collect_assigned_target_variables(&target, names);
        }
    }
}

/// Collect local variable names assigned in a condition node (recursively).
/// Handles boolean operators, parentheses, direct local writes, and destructuring
/// assignments that introduce local targets inside the condition.
fn collect_assigned_variables(node: &ruby_prism::Node<'_>, names: &mut Vec<Vec<u8>>) {
    if let Some(write) = node.as_local_variable_write_node() {
        names.push(write.name().as_slice().to_vec());
    } else if let Some(multi_write) = node.as_multi_write_node() {
        for target in multi_write.lefts().iter() {
            collect_assigned_target_variables(&target, names);
        }
        if let Some(rest) = multi_write.rest() {
            if let Some(splat) = rest.as_splat_node() {
                if let Some(expr) = splat.expression() {
                    collect_assigned_target_variables(&expr, names);
                }
            }
        }
        for target in multi_write.rights().iter() {
            collect_assigned_target_variables(&target, names);
        }
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
    } else if let Some(call) = node.as_call_node() {
        // Descend into call receiver and arguments to find embedded assignments
        // e.g., (locale = foo) != bar has an assignment in the call's receiver
        if let Some(recv) = call.receiver() {
            collect_assigned_variables(&recv, names);
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                collect_assigned_variables(&arg, names);
            }
        }
    }
}

/// ## Corpus investigation (2026-03-12)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// First attempt: replaced the narrow assignment collector with a full visitor so
/// assignments nested inside comparison calls and multi-write nodes also suppress
/// the offense. This introduced 2 new FNs (missing=44 -> missing=46). Reverted.
/// One corpus example in `ruby/rdoc` reduced to a case where RuboCop still fires,
/// so it is not a real FP.
///
/// Second attempt (2026-03-14): targeted fix — added `CallNode` descent to
/// `collect_assigned_variables` so patterns like `(locale = foo) != bar` are
/// recognized. This fixes the refinery/refinerycms FP without the broad visitor
/// that caused the previous FN regression.
///
/// Third attempt (2026-03-14): added `MultiWriteNode` / `MultiTargetNode`
/// target collection so destructuring assignments in the outer condition also
/// suppress the offense when the inner conditional reads that assigned local.
/// This fixes the remaining `ruby/rdoc` FP pattern
/// `if options && (value, = options['value']); ... if value`.
/// Post-fix quick corpus gate: expected=1904, actual=1911, excess=7, missing=0.
pub struct SoleNestedConditional;

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
