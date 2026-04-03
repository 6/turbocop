use crate::cop::shared::node_type::{IF_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-23):
///
/// FN=532 root cause: the original implementation only detected `!` prefix and `not` keyword
/// as negated conditions. RuboCop also treats `!=` and `!~` operators as negated conditions
/// (NEGATED_EQUALITY_METHODS). This was the dominant source of false negatives since `!=` is
/// extremely common in real-world Ruby code.
///
/// Additional FN sources:
/// - Parenthesized conditions like `if (!x)` or `if (x != y)` were not unwrapped
/// - `begin..end` wrapped conditions were not unwrapped
/// - Empty if-branch with non-empty else-branch was rejected (RuboCop flags these)
///
/// FP=13 root cause: double negation `!!x` was not excluded. RuboCop has a `double_negation?`
/// matcher that skips `(send (send _ :!) :!)` patterns. Also, the empty else-branch case
/// `if !x; foo; else; end` was incorrectly flagged (RuboCop requires the else branch to
/// have content).
///
/// Additional guard: RuboCop skips `!=`/`!~` with multiple arguments (e.g., `foo.!=(bar, baz)`).
///
/// Corpus investigation (2026-03-30):
///
/// FN=2 root cause: Prism parses `unless ... else ... end` as `UnlessNode`, but this cop only
/// subscribed to `IF_NODE`, so RuboCop matches like `unless !File.exists?(src) ... else ... end`
/// were never visited. RuboCop sees these through `on_if` because Parser normalizes `unless`
/// into `if`-like nodes. Fix: add `UNLESS_NODE` handling, but only when the `else` branch has
/// content, preserving RuboCop's no-offense behavior for bare `unless !... end` and
/// `unless !... else end`.
pub struct NegatedIfElseCondition;

/// Unwrap parentheses and begin nodes from a condition.
/// RuboCop's `unwrap_begin_nodes` unwraps `:begin` and `:kwbegin` types;
/// in Prism, parenthesized expressions are `ParenthesesNode` and explicit
/// `begin..end` blocks are `BeginNode`.
fn unwrap_condition(node: ruby_prism::Node<'_>) -> ruby_prism::Node<'_> {
    let mut current = node;
    loop {
        if let Some(paren) = current.as_parentheses_node() {
            if let Some(body) = paren.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let stmts_body = stmts.body();
                    if stmts_body.len() == 1 {
                        current = stmts_body.iter().next().unwrap();
                        continue;
                    }
                }
            }
            break;
        } else if let Some(begin) = current.as_begin_node() {
            if let Some(stmts) = begin.statements() {
                let stmts_body = stmts.body();
                if stmts_body.len() == 1 {
                    current = stmts_body.iter().next().unwrap();
                    continue;
                }
            }
            break;
        } else {
            break;
        }
    }
    current
}

/// Check if the node is a double negation: `!!x` i.e. `(send (send _ :!) :!)`
fn is_double_negation(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"!" {
            if let Some(receiver) = call.receiver() {
                if let Some(inner_call) = receiver.as_call_node() {
                    if inner_call.name().as_slice() == b"!" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Check if the condition is negated: `!x`, `not x`, `x != y`, `x !~ y`
fn is_negated(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name_bytes = call.name().as_slice();

        // `!` prefix (unary negation)
        if name_bytes == b"!" {
            return true;
        }

        // `not` keyword
        if let Some(msg_loc) = call.message_loc() {
            if msg_loc.as_slice() == b"not" {
                return true;
            }
        }

        // `!=` and `!~` operators
        if name_bytes == b"!=" || name_bytes == b"!~" {
            // Skip if more than one argument (e.g., foo.!=(bar, baz))
            if let Some(args) = call.arguments() {
                if args.arguments().len() >= 2 {
                    return false;
                }
            }
            return true;
        }
    }
    false
}

fn else_has_content(else_node: &ruby_prism::ElseNode<'_>) -> bool {
    else_node
        .statements()
        .is_some_and(|stmts| !stmts.body().is_empty())
}

impl Cop for NegatedIfElseCondition {
    fn name(&self) -> &'static str {
        "Style/NegatedIfElseCondition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(if_node) = node.as_if_node() {
            // Must have an else/subsequent branch
            let Some(sub) = if_node.subsequent() else {
                return;
            };

            // Determine if ternary (no if_keyword_loc in Prism) or regular if
            let is_ternary = if_node.if_keyword_loc().is_none();

            if !is_ternary {
                let kw = if_node.if_keyword_loc().unwrap();
                let kw_bytes = kw.as_slice();
                // Must be `if`, not `unless` or `elsif`
                if kw_bytes == b"unless" || kw_bytes == b"elsif" {
                    return;
                }
            }

            // Check the subsequent is a plain else (not elsif).
            // If the subsequent is an IfNode, it's an elsif chain - skip.
            if sub.as_if_node().is_some() {
                return;
            }
            // Must be an ElseNode for simple if-else
            let Some(else_node) = sub.as_else_node() else {
                return;
            };

            // Empty else: `if !x; foo; else; end` — not flagged.
            // Empty if with non-empty else: `if !x; else; foo; end` — IS flagged.
            // Both empty: `if !x; else; end` — not flagged.
            if !else_has_content(&else_node) {
                return;
            }

            // Unwrap parentheses/begin nodes from the predicate
            let predicate = if_node.predicate();
            let unwrapped = unwrap_condition(predicate);

            // Skip double negation `!!x`
            if is_double_negation(&unwrapped) {
                return;
            }

            if is_negated(&unwrapped) {
                let loc = if_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let msg = if is_ternary {
                    "Invert the negated condition and swap the ternary branches."
                } else {
                    "Invert the negated condition and swap the if-else branches."
                };
                diagnostics.push(self.diagnostic(source, line, column, msg.to_string()));
            }
            return;
        }

        let Some(unless_node) = node.as_unless_node() else {
            return;
        };

        let Some(else_node) = unless_node.else_clause() else {
            return;
        };

        if !else_has_content(&else_node) {
            return;
        }

        let predicate = unless_node.predicate();
        let unwrapped = unwrap_condition(predicate);

        if is_double_negation(&unwrapped) {
            return;
        }

        if is_negated(&unwrapped) {
            let loc = unless_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Invert the negated condition and swap the if-else branches.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NegatedIfElseCondition,
        "cops/style/negated_if_else_condition"
    );
}
