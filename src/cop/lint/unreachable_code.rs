use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Investigation (2026-03-03)
///
/// Found 1 FP: FactoryBot `fail { false }` treated as `Kernel#fail`
/// (flow-breaking). `Kernel#fail` never accepts blocks, so any `fail`/`raise`
/// with a block is a DSL method call. Fixed by adding `call.block().is_none()`
/// check to `is_raise_call` (dc856393).
///
/// ## Investigation (2026-03-17)
///
/// FP=1: `retry` at top level treated as flow-breaking. RuboCop with prism
/// parser skips `retry` tests entirely (spec line 19). Removed `retry` from
/// flow-breaking nodes since prism doesn't support it outside `begin/rescue`.
///
/// FN=89: Missing detection of several patterns:
/// 1. `throw`, `exit`, `exit!`, `abort` not recognized as flow-breaking calls
/// 2. `redo` not recognized as flow-breaking
/// 3. Code after `if/else` or `case/when/else` where ALL branches break flow
///    was not detected as unreachable
/// 4. Code after `begin..end` blocks containing flow-breaking statements
///    was not detected (e.g., `next` inside `begin..end` in `case/when`)
///
/// Fixed by adding all missing flow-breaking keywords/methods and implementing
/// recursive `flow_expression?` logic matching RuboCop's approach: an if/else
/// is flow-breaking if both branches are, case/when/else is flow-breaking if
/// all branches (including else) are, and begin blocks are flow-breaking if
/// any contained expression is flow-breaking.
///
/// ## Investigation (2026-03-17, second pass)
///
/// FP=228: `begin..rescue..end` blocks were treated as flow-breaking because
/// `flow_expression()` checked if ANY statement in the begin body breaks flow.
/// But when a rescue clause is present, the rescue provides an alternate
/// execution path — the exception is caught, and code after the begin block
/// continues. Fixed by returning false from `flow_expression()` when the
/// `BeginNode` has a `rescue_clause()`.
///
/// FN=28: Mostly `break 1`/`next 1` at top level in spec files (ruby-formatter/rufo).
/// Not addressed in this pass.
///
/// ## Investigation (2026-03-17, third pass)
///
/// FP=19: `begin..ensure..end` blocks were treated as flow-breaking because
/// `flow_expression()` only checked for `rescue_clause` but not `ensure_clause`.
/// A `begin..ensure..end` block is not flow-breaking for the same conservative
/// reason as `begin..rescue..end`: RuboCop does not recurse into begin blocks
/// with ensure clauses to determine flow. Fixed by also returning false from
/// `flow_expression()` when the `BeginNode` has an `ensure_clause()`.
///
/// FN=26: Changed from flagging only the first unreachable statement to using
/// `each_cons(2)` style matching RuboCop's `on_begin`: for each consecutive
/// pair of statements, if the first is flow-breaking, flag the second. This
/// means multiple consecutive flow commands (e.g., `break; break; break`) each
/// get flagged individually.
///
/// Remaining FP=4: method redefinition awareness. RuboCop tracks `def abort`,
/// `def raise`, etc. in `@redefined` and doesn't treat those calls as
/// flow-breaking if redefined in scope (e.g., Spork's `def abort` and
/// EventMachine's `def abort(reason)`). Also tracks `instance_eval` context
/// to suppress warnings inside `instance_eval` blocks. Not yet implemented.
pub struct UnreachableCode;

impl Cop for UnreachableCode {
    fn name(&self) -> &'static str {
        "Lint/UnreachableCode"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = UnreachableVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct UnreachableVisitor<'a, 'src> {
    cop: &'a UnreachableCode,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

/// Check if a node is a simple flow-breaking statement (return, break, next, etc.)
fn is_flow_command(node: &ruby_prism::Node<'_>) -> bool {
    node.as_return_node().is_some()
        || node.as_break_node().is_some()
        || node.as_next_node().is_some()
        || node.as_redo_node().is_some()
        || is_flow_call(node)
}

/// Check if a node is a flow-breaking method call (raise, fail, throw, exit, exit!, abort)
fn is_flow_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        // Kernel#raise/fail never accept blocks — a block means this is a DSL
        // method call (e.g. FactoryBot `fail { false }`), not flow-breaking.
        if call.block().is_some() {
            return false;
        }
        // Only bare calls or calls on Kernel are flow-breaking
        match call.receiver() {
            None => {
                matches!(
                    name,
                    b"raise" | b"fail" | b"throw" | b"exit" | b"exit!" | b"abort"
                )
            }
            Some(recv) => {
                // Kernel.raise, Kernel.exit, etc.
                let is_kernel = if let Some(cr) = recv.as_constant_read_node() {
                    cr.name().as_slice() == b"Kernel"
                } else if let Some(cp) = recv.as_constant_path_node() {
                    // ::Kernel.raise — parent is None (root), child is "Kernel"
                    cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"Kernel")
                } else {
                    false
                };
                if is_kernel {
                    return matches!(
                        name,
                        b"raise" | b"fail" | b"throw" | b"exit" | b"exit!" | b"abort"
                    );
                }
                false
            }
        }
    } else {
        false
    }
}

/// Recursively check if an expression always breaks flow.
/// This matches RuboCop's `flow_expression?` method:
/// - Simple flow commands (return, break, next, redo, raise, fail, throw, exit, etc.)
/// - `if/else` where BOTH branches break flow
/// - `case/when/else` where ALL branches (including else) break flow
/// - `begin`/`kwbegin` where ANY expression breaks flow (since it stops execution)
fn flow_expression(node: &ruby_prism::Node<'_>) -> bool {
    if is_flow_command(node) {
        return true;
    }

    // if/elsif/else: flow-breaking if both if-branch and else-branch break flow
    if let Some(if_node) = node.as_if_node() {
        return check_if_flow(&if_node);
    }

    // unless: also an IfNode in Prism (just with different structure)
    // Prism represents `unless` as an IfNode too, so covered above.

    // case/when/else
    if let Some(case_node) = node.as_case_node() {
        return check_case_flow(&case_node);
    }

    // case/in pattern matching
    if let Some(case_match) = node.as_case_match_node() {
        return check_case_match_flow(&case_match);
    }

    // begin..end (explicit)
    // A begin..rescue..end or begin..ensure..end is NOT flow-breaking because
    // rescue provides an alternate path and RuboCop conservatively treats begin
    // blocks with ensure as non-flow-breaking.
    if let Some(begin_node) = node.as_begin_node() {
        if begin_node.rescue_clause().is_some() || begin_node.ensure_clause().is_some() {
            return false;
        }
        if let Some(stmts) = begin_node.statements() {
            return stmts.body().iter().any(|s| flow_expression(&s));
        }
    }

    // parenthesized expression
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(stmts) = parens.body() {
            if let Some(stmts_node) = stmts.as_statements_node() {
                return stmts_node.body().iter().any(|s| flow_expression(&s));
            }
        }
    }

    false
}

fn check_if_flow(node: &ruby_prism::IfNode<'_>) -> bool {
    let if_branch = node.statements();
    let else_branch = node.subsequent();

    // Must have both branches
    let Some(if_stmts) = if_branch else {
        return false;
    };
    let Some(else_clause) = else_branch else {
        return false;
    };

    // Check if-branch: any statement in the if body is flow-breaking
    let if_breaks = if_stmts.body().iter().any(|s| flow_expression(&s));
    if !if_breaks {
        return false;
    }

    // Check else-branch: could be ElseNode or another IfNode (elsif)
    if let Some(else_node) = else_clause.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            return stmts.body().iter().any(|s| flow_expression(&s));
        }
        return false;
    }
    if let Some(ref elsif_node) = else_clause.as_if_node() {
        return check_if_flow(elsif_node);
    }
    false
}

fn check_case_flow(node: &ruby_prism::CaseNode<'_>) -> bool {
    // Must have an else branch
    let Some(else_clause) = node.else_clause() else {
        return false;
    };
    // Check else branch
    if let Some(stmts) = else_clause.statements() {
        if !stmts.body().iter().any(|s| flow_expression(&s)) {
            return false;
        }
    } else {
        return false;
    }

    // Check all when branches
    let conditions = node.conditions();
    if conditions.is_empty() {
        return false;
    }
    for condition in conditions.iter() {
        if let Some(when_node) = condition.as_when_node() {
            if let Some(stmts) = when_node.statements() {
                if !stmts.body().iter().any(|s| flow_expression(&s)) {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn check_case_match_flow(node: &ruby_prism::CaseMatchNode<'_>) -> bool {
    // Must have an else branch
    let Some(else_clause) = node.else_clause() else {
        return false;
    };
    if let Some(stmts) = else_clause.statements() {
        if !stmts.body().iter().any(|s| flow_expression(&s)) {
            return false;
        }
    } else {
        return false;
    }

    // Check all in branches
    let conditions = node.conditions();
    if conditions.is_empty() {
        return false;
    }
    for condition in conditions.iter() {
        if let Some(in_node) = condition.as_in_node() {
            if let Some(stmts) = in_node.statements() {
                if !stmts.body().iter().any(|s| flow_expression(&s)) {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

impl<'pr> Visit<'pr> for UnreachableVisitor<'_, '_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let body: Vec<_> = node.body().iter().collect();

        // Match RuboCop's each_cons(2) approach: for each consecutive pair,
        // if the first expression is flow-breaking, flag the second.
        for pair in body.windows(2) {
            if flow_expression(&pair[0]) {
                let loc = pair[1].location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Unreachable code detected.".to_string(),
                ));
            }
        }

        ruby_prism::visit_statements_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnreachableCode, "cops/lint/unreachable_code");
}
