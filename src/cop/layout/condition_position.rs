use crate::cop::shared::node_type::{IF_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks that statement-form conditional keywords keep their condition on the
/// same line as the keyword.
///
/// 2026-03-30:
/// FN=1. Prism parses statement-form `unless` as `UnlessNode`, not `IfNode`.
/// This cop only listened for `IfNode`, `WhileNode`, and `UntilNode`, so a
/// multiline `unless` condition from the corpus was never visited. Fix: add
/// `UNLESS_NODE` and apply the same predicate-line check used for the other
/// statement-form conditionals while still skipping modifier `unless`.
pub struct ConditionPosition;

impl Cop for ConditionPosition {
    fn name(&self) -> &'static str {
        "Layout/ConditionPosition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE]
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
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return,
            };
            let keyword = if kw_loc.as_slice() == b"if" {
                "if"
            } else if kw_loc.as_slice() == b"unless" {
                "unless"
            } else {
                // elsif — keyword_loc is "elsif"; end_keyword_loc is None
                // for elsif nodes (the end belongs to the outermost if)
                "elsif"
            };
            // Skip modifier form (postfix if/unless) — no `end` keyword and
            // not an elsif (which also lacks end_keyword_loc).
            if if_node.end_keyword_loc().is_none() && keyword != "elsif" {
                return;
            }
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let predicate = if_node.predicate();
            let (pred_line, pred_col) =
                source.offset_to_line_col(predicate.location().start_offset());
            if pred_line != kw_line {
                diagnostics.push(self.diagnostic(
                    source,
                    pred_line,
                    pred_col,
                    format!("Place the condition on the same line as `{keyword}`."),
                ));
            }
        } else if let Some(unless_node) = node.as_unless_node() {
            // Skip modifier form (postfix unless) — no closing `end` keyword
            if unless_node.end_keyword_loc().is_none() {
                return;
            }
            let kw_loc = unless_node.keyword_loc();
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let predicate = unless_node.predicate();
            let (pred_line, pred_col) =
                source.offset_to_line_col(predicate.location().start_offset());
            if pred_line != kw_line {
                diagnostics.push(self.diagnostic(
                    source,
                    pred_line,
                    pred_col,
                    "Place the condition on the same line as `unless`.".to_string(),
                ));
            }
        } else if let Some(while_node) = node.as_while_node() {
            // Skip modifier form (postfix while) — no closing `end` keyword
            if while_node.closing_loc().is_none() {
                return;
            }
            let kw_loc = while_node.keyword_loc();
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let predicate = while_node.predicate();
            let (pred_line, pred_col) =
                source.offset_to_line_col(predicate.location().start_offset());
            if pred_line != kw_line {
                diagnostics.push(self.diagnostic(
                    source,
                    pred_line,
                    pred_col,
                    "Place the condition on the same line as `while`.".to_string(),
                ));
            }
        } else if let Some(until_node) = node.as_until_node() {
            // Skip modifier form (postfix until) — no closing `end` keyword
            if until_node.closing_loc().is_none() {
                return;
            }
            let kw_loc = until_node.keyword_loc();
            let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
            let predicate = until_node.predicate();
            let (pred_line, pred_col) =
                source.offset_to_line_col(predicate.location().start_offset());
            if pred_line != kw_line {
                diagnostics.push(self.diagnostic(
                    source,
                    pred_line,
                    pred_col,
                    "Place the condition on the same line as `until`.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(ConditionPosition, "cops/layout/condition_position");

    #[test]
    fn inline_if_no_offense() {
        let source = b"x = 1 if true\n";
        let diags = run_cop_full(&ConditionPosition, source);
        assert!(diags.is_empty());
    }
}
