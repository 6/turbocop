use crate::cop::node_type::{CALL_NODE, UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/NegatedWhile flags `while !condition` / `while not condition` and suggests
/// `until`, and also flags `until !condition` / `until not condition` suggesting `while`.
///
/// Key behaviors matching RuboCop's NegativeConditional mixin:
/// - Unwraps parentheses around the condition before checking for negation
///   (e.g. `while (!foo)` and `while (not bar)` are flagged)
/// - Skips double negation `!!` (not a true negation, it's a boolean cast)
/// - Skips safe-navigation chains ending in `&.!` (rewriting is problematic)
/// - Handles both prefix and modifier (postfix) forms
/// - Handles both `while` and `until` nodes
///
/// Root causes of prior FPs/FNs:
/// - FNs: `not` keyword was not detected (Prism parses `not expr` as CallNode
///   with name `!`, same as `!expr`, so it was actually the parentheses issue)
/// - FNs: parenthesized conditions `while(!cond)` and `while (not cond)` were
///   missed because `predicate.as_call_node()` returned None for the
///   ParenthesesNode wrapper
/// - FNs: `until !condition` was not handled at all (cop only checked WhileNode)
/// - FPs: `!!condition` double negation was not excluded
/// - FPs: safe-navigation chains `&.!` were not excluded
/// - FPs: `begin...end while !cond` (do-while loops) were flagged but RuboCop
///   skips these entirely. Fixed by checking `is_begin_modifier()` on both
///   WhileNode and UntilNode, consistent with other cops (e.g. Lint/Loop).
pub struct NegatedWhile;

/// Unwrap parentheses from a node, returning the inner expression.
/// Handles `(expr)`, `((expr))`, etc.
fn unwrap_parentheses<'a>(node: ruby_prism::Node<'a>) -> ruby_prism::Node<'a> {
    let mut current = node;
    while let Some(paren) = current.as_parentheses_node() {
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
    }
    current
}

/// Check if a node is a single negation (`!expr` or `not expr`),
/// excluding double negation (`!!expr`) and safe-navigation `&.!`.
fn is_single_negation(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"!" {
            // Skip safe-navigation `&.!` — rewriting is problematic
            if call.call_operator_loc().is_some() {
                return false;
            }
            // Check for double negation: `!!expr`
            if let Some(recv) = call.receiver() {
                if let Some(inner_call) = recv.as_call_node() {
                    if inner_call.name().as_slice() == b"!" {
                        return false;
                    }
                }
            }
            return true;
        }
    }
    false
}

/// Get the inner expression from a negation node (`!expr` → `expr`).
fn get_negation_inner<'a>(node: &ruby_prism::Node<'a>) -> Option<ruby_prism::Node<'a>> {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"!" {
            return call.receiver();
        }
    }
    None
}

/// Add corrections for negated loop: swap keyword and remove negation.
fn add_negated_loop_corrections(
    cop: &NegatedWhile,
    kw_loc: &ruby_prism::Location<'_>,
    predicate: &ruby_prism::Node<'_>,
    unwrapped: &ruby_prism::Node<'_>,
    replacement_keyword: &str,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    diag: &mut Diagnostic,
) {
    if let Some(corr) = corrections {
        // 1. Replace keyword
        corr.push(crate::correction::Correction {
            start: kw_loc.start_offset(),
            end: kw_loc.end_offset(),
            replacement: replacement_keyword.to_string(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        // 2. Replace predicate with inner expression (without negation/parens)
        if let Some(inner) = get_negation_inner(unwrapped) {
            let inner_src = std::str::from_utf8(inner.location().as_slice())
                .unwrap_or("")
                .to_string();
            let pred_start = predicate.location().start_offset();
            let pred_end = predicate.location().end_offset();
            // Add space if keyword and predicate are adjacent (no space)
            let needs_space = pred_start == kw_loc.end_offset();
            let replacement = if needs_space {
                format!(" {inner_src}")
            } else {
                inner_src
            };
            corr.push(crate::correction::Correction {
                start: pred_start,
                end: pred_end,
                replacement,
                cop_name: cop.name(),
                cop_index: 0,
            });
        }
        diag.corrected = true;
    }
}

impl Cop for NegatedWhile {
    fn name(&self) -> &'static str {
        "Style/NegatedWhile"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, WHILE_NODE, UNTIL_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Handle WhileNode: `while !cond` -> suggest `until`
        if let Some(while_node) = node.as_while_node() {
            // Skip begin...end while loops (do-while); RuboCop does not flag these
            if while_node.is_begin_modifier() {
                return;
            }
            let predicate = while_node.predicate();
            let unwrapped = unwrap_parentheses(predicate);
            if is_single_negation(&unwrapped) {
                let (line, column) = source.offset_to_line_col(node.location().start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Favor `until` over `while` for negative conditions.".to_string(),
                );
                add_negated_loop_corrections(
                    self,
                    &while_node.keyword_loc(),
                    &while_node.predicate(),
                    &unwrapped,
                    "until",
                    &mut corrections,
                    &mut diag,
                );
                diagnostics.push(diag);
            }
            return;
        }

        // Handle UntilNode: `until !cond` -> suggest `while`
        if let Some(until_node) = node.as_until_node() {
            // Skip begin...end until loops (do-until); RuboCop does not flag these
            if until_node.is_begin_modifier() {
                return;
            }
            let predicate = until_node.predicate();
            let unwrapped = unwrap_parentheses(predicate);
            if is_single_negation(&unwrapped) {
                let (line, column) = source.offset_to_line_col(node.location().start_offset());
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Favor `while` over `until` for negative conditions.".to_string(),
                );
                add_negated_loop_corrections(
                    self,
                    &until_node.keyword_loc(),
                    &until_node.predicate(),
                    &unwrapped,
                    "while",
                    &mut corrections,
                    &mut diag,
                );
                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(NegatedWhile, "cops/style/negated_while");
    crate::cop_autocorrect_fixture_tests!(NegatedWhile, "cops/style/negated_while");

    #[test]
    fn parenthesized_negation() {
        use crate::testutil::run_cop_full;
        let source = b"while (!foo)\n  bar\nend\n";
        let diags = run_cop_full(&NegatedWhile, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag parenthesized negation: {:?}",
            diags
        );
    }

    #[test]
    fn not_keyword() {
        use crate::testutil::run_cop_full;
        let source = b"while not condition\n  do_something\nend\n";
        let diags = run_cop_full(&NegatedWhile, source);
        assert_eq!(diags.len(), 1, "Should flag 'not' keyword: {:?}", diags);
    }

    #[test]
    fn double_negation_not_flagged() {
        use crate::testutil::run_cop_full;
        let source = b"while !!condition\n  do_something\nend\n";
        let diags = run_cop_full(&NegatedWhile, source);
        assert_eq!(
            diags.len(),
            0,
            "Should NOT flag double negation: {:?}",
            diags
        );
    }

    #[test]
    fn safe_nav_chain_not_flagged() {
        use crate::testutil::run_cop_full;
        let source = b"while obj&.empty?&.!\n  do_something\nend\n";
        let diags = run_cop_full(&NegatedWhile, source);
        assert_eq!(
            diags.len(),
            0,
            "Should NOT flag safe-nav chain: {:?}",
            diags
        );
    }

    #[test]
    fn until_negated_flagged() {
        use crate::testutil::run_cop_full;
        let source = b"until !done?\n  process\nend\n";
        let diags = run_cop_full(&NegatedWhile, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag until with negation: {:?}",
            diags
        );
        assert!(diags[0].message.contains("while"), "Should suggest 'while'");
    }

    #[test]
    fn modifier_until_negated() {
        use crate::testutil::run_cop_full;
        let source = b"x += 1 until !list.include?(x)\n";
        let diags = run_cop_full(&NegatedWhile, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag modifier until with negation: {:?}",
            diags
        );
    }
}
