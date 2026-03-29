use crate::cop::node_type::{IF_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for assignments in the conditions of if/while/until/unless.
///
/// ## Root causes of FNs (577):
/// - Only checked direct predicate, not nested assignments in `||`/`&&` conditions
/// - Missing assignment method support (`obj.method = 10`, `a[3] = 10`, `obj&.method = 10`)
/// - Missing safe navigation assignment methods
/// - Not traversing into compound conditions
///
/// ## Root causes of FPs (4):
/// - Message format differed from RuboCop
/// - Offense location was on entire assignment instead of just the `=` operator
/// - `begin..end while/until cond` (while_post/until_post in parser gem) should not
///   be flagged — RuboCop's `on_while` alias only fires for regular `while`, not
///   `while_post`. In Prism both are `WhileNode` but distinguished by
///   `is_begin_modifier()`. Fix: skip when `is_begin_modifier()` is true.
///
/// ## Corpus investigation (2026-03-23)
///
/// Corpus oracle reported FP=0, FN=3.
///
/// FN=3: All 3 from newrelic/newrelic-ruby-agent rake_test.rb. The flagged lines
/// are plain assignments in method bodies (`trace = single_transaction_trace_posted`,
/// `expected = [...]`, `event = single_event_posted[0]`), NOT inside conditions.
/// These are corpus oracle artifacts — RuboCop should not flag these, and nitrocop
/// correctly does not. No code change needed.
///
/// ## Corpus follow-up (2026-03-29)
///
/// Reproduced the real `newrelic` method-body snippet under both RuboCop and
/// nitrocop with 0 offenses. The failing local fixture was caused by a bad test
/// change that copied the oracle line numbers into `offense.rb` as standalone
/// assignments. Keep detector behavior unchanged, cover the snippet in
/// `no_offense.rb`, and treat the remaining CI FN entries as stale oracle data.
///
/// ## FN fix (2026-03-28): recurse into assignment values
///
/// Corpus oracle reported FP=0, FN=7 (3 oracle artifacts from above + 1 config
/// issue + 3 real code bugs). The 3 code bugs were all the same root cause:
/// `traverse_condition` reported an assignment and returned immediately, without
/// recursing into the assignment's value. This missed nested assignments like:
/// - `if x = foo && y = bar` (parsed as `x = (foo && (y = bar))`)
/// - `if klass = begin; inner = ...; end`
/// - `if a && b = c && d = e` (triple chain)
///
/// Fix: after reporting an equals or call assignment, recurse into the value node.
/// This mirrors RuboCop's `traverse_node` which unconditionally walks all children.
///
/// ## Fix:
/// Rewrote to use recursive condition traversal matching RuboCop's `traverse_node`:
/// - Recursively walks condition tree finding assignments at any depth
/// - Handles CallNode assignment methods (`is_attribute_write()`)
/// - Skips block nodes (assignments inside blocks are irrelevant)
/// - Skips conditional assignments (`||=`, `&&=`)
/// - Handles `AllowSafeAssignment` via parenthesized assignment detection
/// - Reports offense on the `=` operator location specifically
///
/// ## FN fix (46 FNs):
/// Added `ConstantPathWriteNode` handling (e.g., `Foo::Bar = 1` in condition).
/// Parser gem's `:casgn` maps to both `ConstantWriteNode` (simple `Foo = 1`)
/// and `ConstantPathWriteNode` (qualified `Foo::Bar = 1`). Only `ConstantWriteNode`
/// was handled. Also added `MultiWriteNode` to `get_equals_assignment_operator_loc`
/// for completeness (it was already in `is_assignment_node` for safe assignment).
///
/// ## FN fix (41 FNs):
/// RuboCop's `traverse_node` unconditionally recurses into ALL child nodes of
/// the condition expression (except blocks). Our `recurse_children` only handled
/// specific node types (Or, And, Statements, Range, Defined). Missing types:
/// - `BeginNode`: explicit `begin..end` blocks in conditions (e.g., `if x && begin; y = z; end`)
///   including rescue/ensure clauses inside begin blocks
/// - `CaseNode`/`WhenNode`: case expressions used as conditions (e.g., `elsif case; when match = scan(...)`)
///   with recursion into both `when` conditions and bodies
/// - `IfNode`/`UnlessNode`/`WhileNode`/`UntilNode`: nested control flow inside condition
///   subtrees (e.g., modifier `if` inside `when` body that's part of an `elsif` condition)
/// - `ElseNode`: else clauses
/// - `RescueNode`: rescue handlers inside begin blocks
/// - `EnsureNode`: ensure clauses inside begin blocks
/// - `RescueModifierNode`: inline rescue (`expr rescue fallback`)
pub struct AssignmentInCondition;

impl Cop for AssignmentInCondition {
    fn name(&self) -> &'static str {
        "Lint/AssignmentInCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE]
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
        let allow_safe = config.get_bool("AllowSafeAssignment", true);

        let predicate = if let Some(if_node) = node.as_if_node() {
            Some(if_node.predicate())
        } else if let Some(while_node) = node.as_while_node() {
            // begin..end while cond is while_post in parser gem — RuboCop's
            // on_while doesn't fire for it, so skip to avoid false positives.
            if while_node.is_begin_modifier() {
                return;
            }
            Some(while_node.predicate())
        } else if let Some(until_node) = node.as_until_node() {
            // Same for begin..end until cond (until_post in parser gem).
            if until_node.is_begin_modifier() {
                return;
            }
            Some(until_node.predicate())
        } else {
            node.as_unless_node()
                .map(|unless_node| unless_node.predicate())
        };

        let predicate = match predicate {
            Some(p) => p,
            None => return,
        };

        let msg = if allow_safe {
            MSG_WITH_SAFE_ASSIGNMENT
        } else {
            MSG_WITHOUT_SAFE_ASSIGNMENT
        };

        traverse_condition(source, &predicate, allow_safe, msg, self, diagnostics);
    }
}

const MSG_WITH_SAFE_ASSIGNMENT: &str = "Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.";
const MSG_WITHOUT_SAFE_ASSIGNMENT: &str =
    "Use `==` if you meant to do a comparison or move the assignment up out of the condition.";

/// Recursively traverses a condition node tree looking for assignments.
/// Mirrors RuboCop's `traverse_node` logic.
fn traverse_condition(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    allow_safe: bool,
    msg: &str,
    cop: &AssignmentInCondition,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Block nodes halt traversal — assignments inside blocks are irrelevant
    if node.as_block_node().is_some() || node.as_lambda_node().is_some() {
        return;
    }

    // Check if this is a parenthesized expression (RuboCop's :begin type)
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            // Check for safe assignment: (assignment)
            if allow_safe && is_assignment_node(&body) {
                // Safe assignment — skip children
                return;
            }
            // If body is a StatementsNode, check its single child
            if allow_safe {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    if body_nodes.len() == 1 && is_assignment_node(&body_nodes[0]) {
                        // Safe assignment — skip children
                        return;
                    }
                }
            }
            // Not a safe assignment, recurse into the body
            traverse_condition(source, &body, allow_safe, msg, cop, diagnostics);
        }
        // Empty parens — nothing to do
        return;
    }

    // Check for equals assignments (lvar, ivar, cvar, gvar, constant write)
    if let Some(op_loc) = get_equals_assignment_operator_loc(node) {
        let (line, column) = source.offset_to_line_col(op_loc);
        diagnostics.push(cop.diagnostic(source, line, column, msg.to_string()));
        // Continue recursing into the value — it may contain nested assignments
        // e.g., `x = foo && y = bar` parses as `x = (foo && (y = bar))`
        if let Some(value) = get_assignment_value(node) {
            traverse_condition(source, &value, allow_safe, msg, cop, diagnostics);
        }
        return;
    }

    // Check for call assignment methods (obj.method = 10, a[3] = 10, obj&.method = 10)
    if let Some(call) = node.as_call_node() {
        if call.is_attribute_write() {
            // This is an assignment method — report on the `=` operator
            if let Some(eq_offset) = find_call_assignment_equals(source, &call) {
                let (line, column) = source.offset_to_line_col(eq_offset);
                diagnostics.push(cop.diagnostic(source, line, column, msg.to_string()));
            }
            // Continue recursing into arguments (the assigned value)
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    traverse_condition(source, &arg, allow_safe, msg, cop, diagnostics);
                }
            }
            return;
        }
        // Non-assignment call — skip children (don't look inside method arguments)
        return;
    }

    // Conditional assignments (||=, &&=) — allowed, skip (RuboCop's conditional_assignment?)
    if is_conditional_or_operator_assignment(node) {
        return;
    }

    // For other node types, recurse into children
    recurse_children(source, node, allow_safe, msg, cop, diagnostics);
}

/// Check if a node is an equals assignment (lvar=, ivar=, cvar=, gvar=, const=)
fn is_assignment_node(node: &ruby_prism::Node<'_>) -> bool {
    node.as_local_variable_write_node().is_some()
        || node.as_instance_variable_write_node().is_some()
        || node.as_class_variable_write_node().is_some()
        || node.as_global_variable_write_node().is_some()
        || node.as_constant_write_node().is_some()
        || node.as_constant_path_write_node().is_some()
        || node.as_multi_write_node().is_some()
        || node.as_call_node().is_some_and(|c| c.is_attribute_write())
}

/// Get the operator location (byte offset of `=`) for equals assignment nodes
fn get_equals_assignment_operator_loc(node: &ruby_prism::Node<'_>) -> Option<usize> {
    if let Some(n) = node.as_local_variable_write_node() {
        return Some(n.operator_loc().start_offset());
    }
    if let Some(n) = node.as_instance_variable_write_node() {
        return Some(n.operator_loc().start_offset());
    }
    if let Some(n) = node.as_class_variable_write_node() {
        return Some(n.operator_loc().start_offset());
    }
    if let Some(n) = node.as_global_variable_write_node() {
        return Some(n.operator_loc().start_offset());
    }
    if let Some(n) = node.as_constant_write_node() {
        return Some(n.operator_loc().start_offset());
    }
    if let Some(n) = node.as_constant_path_write_node() {
        return Some(n.operator_loc().start_offset());
    }
    if let Some(n) = node.as_multi_write_node() {
        return Some(n.operator_loc().start_offset());
    }
    None
}

/// Get the value (right-hand side) of an equals assignment node.
/// Used to recurse into the value after reporting the assignment offense.
fn get_assignment_value<'a>(node: &ruby_prism::Node<'a>) -> Option<ruby_prism::Node<'a>> {
    if let Some(n) = node.as_local_variable_write_node() {
        return Some(n.value());
    }
    if let Some(n) = node.as_instance_variable_write_node() {
        return Some(n.value());
    }
    if let Some(n) = node.as_class_variable_write_node() {
        return Some(n.value());
    }
    if let Some(n) = node.as_global_variable_write_node() {
        return Some(n.value());
    }
    if let Some(n) = node.as_constant_write_node() {
        return Some(n.value());
    }
    if let Some(n) = node.as_constant_path_write_node() {
        return Some(n.value());
    }
    if let Some(n) = node.as_multi_write_node() {
        return Some(n.value());
    }
    None
}

/// Check if node is a conditional assignment (||=, &&=) or operator assignment (+=, etc.)
fn is_conditional_or_operator_assignment(node: &ruby_prism::Node<'_>) -> bool {
    node.as_local_variable_or_write_node().is_some()
        || node.as_local_variable_and_write_node().is_some()
        || node.as_instance_variable_or_write_node().is_some()
        || node.as_instance_variable_and_write_node().is_some()
        || node.as_class_variable_or_write_node().is_some()
        || node.as_class_variable_and_write_node().is_some()
        || node.as_global_variable_or_write_node().is_some()
        || node.as_global_variable_and_write_node().is_some()
        || node.as_constant_or_write_node().is_some()
        || node.as_constant_and_write_node().is_some()
        || node.as_constant_path_or_write_node().is_some()
        || node.as_constant_path_and_write_node().is_some()
        || node.as_local_variable_operator_write_node().is_some()
        || node.as_instance_variable_operator_write_node().is_some()
        || node.as_class_variable_operator_write_node().is_some()
        || node.as_global_variable_operator_write_node().is_some()
        || node.as_constant_operator_write_node().is_some()
        || node.as_constant_path_operator_write_node().is_some()
}

/// Find the byte offset of the `=` sign in a call assignment method.
/// For `obj.method = 10`, the `=` is after message_loc.
/// For `a[3] = 10`, the `=` is after closing_loc (`]`).
fn find_call_assignment_equals(
    source: &SourceFile,
    call: &ruby_prism::CallNode<'_>,
) -> Option<usize> {
    let bytes = source.as_bytes();

    // Determine the search start position: after closing_loc for []=, after message_loc otherwise
    let search_start = if let Some(closing) = call.closing_loc() {
        // []=  method: search after the `]`
        closing.end_offset()
    } else if let Some(msg) = call.message_loc() {
        // setter method: search after the method name
        msg.end_offset()
    } else {
        return None;
    };

    // Scan forward from search_start to find `=`
    let mut pos = search_start;
    while pos < bytes.len() {
        if bytes[pos] == b'=' {
            return Some(pos);
        }
        if bytes[pos] != b' ' && bytes[pos] != b'\t' {
            break;
        }
        pos += 1;
    }
    None
}

/// Recurse into child nodes of common condition expression types.
fn recurse_children(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    allow_safe: bool,
    msg: &str,
    cop: &AssignmentInCondition,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // OrNode (||)
    if let Some(or_node) = node.as_or_node() {
        traverse_condition(source, &or_node.left(), allow_safe, msg, cop, diagnostics);
        traverse_condition(source, &or_node.right(), allow_safe, msg, cop, diagnostics);
        return;
    }
    // AndNode (&&)
    if let Some(and_node) = node.as_and_node() {
        traverse_condition(source, &and_node.left(), allow_safe, msg, cop, diagnostics);
        traverse_condition(source, &and_node.right(), allow_safe, msg, cop, diagnostics);
        return;
    }
    // StatementsNode — multiple statements in a begin block
    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
        }
        return;
    }
    // RangeNode (flip-flop conditions)
    if let Some(range) = node.as_range_node() {
        if let Some(left) = range.left() {
            traverse_condition(source, &left, allow_safe, msg, cop, diagnostics);
        }
        if let Some(right) = range.right() {
            traverse_condition(source, &right, allow_safe, msg, cop, diagnostics);
        }
        return;
    }
    // DefinedNode
    if let Some(defined) = node.as_defined_node() {
        traverse_condition(source, &defined.value(), allow_safe, msg, cop, diagnostics);
        return;
    }
    // BeginNode — explicit begin..end blocks used inside conditions
    // e.g., `if valid? && begin; result = compute; result.present?; end`
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for stmt in stmts.body().iter() {
                traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
            }
        }
        // Also traverse rescue/ensure clauses inside begin blocks
        if let Some(rescue) = begin_node.rescue_clause() {
            traverse_condition(source, &rescue.as_node(), allow_safe, msg, cop, diagnostics);
        }
        if let Some(ensure) = begin_node.ensure_clause() {
            traverse_condition(source, &ensure.as_node(), allow_safe, msg, cop, diagnostics);
        }
        return;
    }
    // CaseNode — case expressions used inside conditions
    // e.g., `if (case x; when :a; found = lookup; end)` or bare `case; when match = scan(...)`
    if let Some(case_node) = node.as_case_node() {
        for condition in case_node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                // Traverse when conditions (the expressions after `when`)
                for cond in when_node.conditions().iter() {
                    traverse_condition(source, &cond, allow_safe, msg, cop, diagnostics);
                }
                // Traverse when body (statements inside the when clause)
                if let Some(stmts) = when_node.statements() {
                    for stmt in stmts.body().iter() {
                        traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
                    }
                }
            }
        }
        // Also check else clause
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for stmt in stmts.body().iter() {
                    traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
                }
            }
        }
        return;
    }
    // IfNode / UnlessNode / WhileNode / UntilNode — when these appear nested
    // inside a condition tree (e.g., `if modifier` inside a `when` body of a `case`
    // that's used as an `elsif` condition), we recurse into both predicate and body.
    // RuboCop's traverse_node walks all children unconditionally.
    if let Some(if_node) = node.as_if_node() {
        traverse_condition(
            source,
            &if_node.predicate(),
            allow_safe,
            msg,
            cop,
            diagnostics,
        );
        if let Some(stmts) = if_node.statements() {
            for stmt in stmts.body().iter() {
                traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            traverse_condition(source, &subsequent, allow_safe, msg, cop, diagnostics);
        }
        return;
    }
    if let Some(unless_node) = node.as_unless_node() {
        traverse_condition(
            source,
            &unless_node.predicate(),
            allow_safe,
            msg,
            cop,
            diagnostics,
        );
        if let Some(stmts) = unless_node.statements() {
            for stmt in stmts.body().iter() {
                traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            traverse_condition(
                source,
                &else_clause.as_node(),
                allow_safe,
                msg,
                cop,
                diagnostics,
            );
        }
        return;
    }
    if let Some(while_node) = node.as_while_node() {
        traverse_condition(
            source,
            &while_node.predicate(),
            allow_safe,
            msg,
            cop,
            diagnostics,
        );
        if let Some(stmts) = while_node.statements() {
            for stmt in stmts.body().iter() {
                traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
            }
        }
        return;
    }
    if let Some(until_node) = node.as_until_node() {
        traverse_condition(
            source,
            &until_node.predicate(),
            allow_safe,
            msg,
            cop,
            diagnostics,
        );
        if let Some(stmts) = until_node.statements() {
            for stmt in stmts.body().iter() {
                traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
            }
        }
        return;
    }
    // ElseNode — else clause of if/unless/case
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for stmt in stmts.body().iter() {
                traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
            }
        }
        return;
    }
    // RescueNode — individual rescue clause (e.g., `rescue => e; stmts`)
    if let Some(rescue_node) = node.as_rescue_node() {
        if let Some(stmts) = rescue_node.statements() {
            for stmt in stmts.body().iter() {
                traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
            }
        }
        // Chain to subsequent rescue clauses
        if let Some(subsequent) = rescue_node.subsequent() {
            traverse_condition(
                source,
                &subsequent.as_node(),
                allow_safe,
                msg,
                cop,
                diagnostics,
            );
        }
        return;
    }
    // EnsureNode
    if let Some(ensure_node) = node.as_ensure_node() {
        if let Some(stmts) = ensure_node.statements() {
            for stmt in stmts.body().iter() {
                traverse_condition(source, &stmt, allow_safe, msg, cop, diagnostics);
            }
        }
        return;
    }
    // RescueModifierNode — `expr rescue fallback` inline rescue
    if let Some(rescue_mod) = node.as_rescue_modifier_node() {
        traverse_condition(
            source,
            &rescue_mod.expression(),
            allow_safe,
            msg,
            cop,
            diagnostics,
        );
        traverse_condition(
            source,
            &rescue_mod.rescue_expression(),
            allow_safe,
            msg,
            cop,
            diagnostics,
        );
    }
    // For other node types we don't recurse — they're leaf nodes or types
    // where assignments aren't relevant (e.g., literals, method args)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AssignmentInCondition, "cops/lint/assignment_in_condition");
}
