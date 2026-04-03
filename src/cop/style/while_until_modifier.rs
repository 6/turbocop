use crate::cop::shared::node_type::{UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Corpus investigation: 30 FPs across 15 repos caused by multi-line conditions
/// (e.g., `while a && \n b`). RuboCop checks both body AND predicate line span
/// before suggesting modifier form. Fixed by adding predicate line-span check
/// after the existing body line-span check.
///
/// Second round (2026-03-08): 19 FPs from missing StatementModifier checks:
/// - `nonempty_line_count > 3`: skip if the node spans more than 3 non-empty lines
///   (catches cases with comment lines between keyword and body)
/// - `line_with_comment?(end_line)`: skip if end keyword's line has a comment
/// - `contains_comment?(body.source_range)`: skip if any comment falls within body range
/// - `code_after(node)`: include trailing code after `end` in modifier length calculation
/// - `first_line_comment && code_after`: skip if keyword line has comment AND end has trailing code
pub struct WhileUntilModifier;

/// Returns true if the node or any descendant contains a local variable assignment.
fn contains_lvar_assignment(node: &ruby_prism::Node<'_>) -> bool {
    struct LvarAssignChecker {
        found: bool,
    }
    impl<'pr> Visit<'pr> for LvarAssignChecker {
        fn visit_local_variable_write_node(
            &mut self,
            _node: &ruby_prism::LocalVariableWriteNode<'pr>,
        ) {
            self.found = true;
        }
        fn visit_local_variable_and_write_node(
            &mut self,
            _node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
        ) {
            self.found = true;
        }
        fn visit_local_variable_or_write_node(
            &mut self,
            _node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
        ) {
            self.found = true;
        }
        fn visit_local_variable_operator_write_node(
            &mut self,
            _node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
        ) {
            self.found = true;
        }
    }
    let mut checker = LvarAssignChecker { found: false };
    checker.visit(node);
    checker.found
}

impl Cop for WhileUntilModifier {
    fn name(&self) -> &'static str {
        "Style/WhileUntilModifier"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[UNTIL_NODE, WHILE_NODE]
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
        let (kw_loc, statements, keyword) = if let Some(while_node) = node.as_while_node() {
            (while_node.keyword_loc(), while_node.statements(), "while")
        } else if let Some(until_node) = node.as_until_node() {
            (until_node.keyword_loc(), until_node.statements(), "until")
        } else {
            return;
        };

        // Skip modifier form — check if keyword comes before the closing
        let closing_loc = if let Some(while_node) = node.as_while_node() {
            while_node.closing_loc()
        } else if let Some(until_node) = node.as_until_node() {
            until_node.closing_loc()
        } else {
            return;
        };

        // If no closing (end), it's already modifier form
        let closing_loc = match closing_loc {
            Some(cl) => cl,
            None => return,
        };

        let body = match statements {
            Some(s) => s,
            None => return,
        };

        let body_stmts: Vec<_> = body.body().iter().collect();
        if body_stmts.len() != 1 {
            return;
        }

        let body_node = &body_stmts[0];
        let src_bytes = source.as_bytes();

        // non_eligible_node?: nonempty_line_count > 3
        // Count non-empty lines in the entire while/until node span
        let node_start = node.location().start_offset();
        let node_end = node.location().end_offset();
        let node_src = &src_bytes[node_start..node_end];
        let nonempty_line_count = node_src
            .split(|&b| b == b'\n')
            .filter(|line| line.iter().any(|&b| b != b' ' && b != b'\t' && b != b'\r'))
            .count();
        if nonempty_line_count > 3 {
            return;
        }

        // non_eligible_node?: comment on end keyword's line
        let (end_line, _) = source.offset_to_line_col(closing_loc.start_offset());
        for comment in _parse_result.comments() {
            let (comment_line, _) = source.offset_to_line_col(comment.location().start_offset());
            if comment_line == end_line {
                return;
            }
        }

        // non_eligible_body?: comment within body source range
        let body_start = body_node.location().start_offset();
        let body_end = body_node.location().end_offset();
        // Extend body_end to include the rest of the line (catches inline comments)
        let body_line_end = src_bytes[body_end..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| body_end + p)
            .unwrap_or(src_bytes.len());
        for comment in _parse_result.comments() {
            let comment_start = comment.location().start_offset();
            if comment_start >= body_start && comment_start < body_line_end {
                return;
            }
        }

        // Body must be on a single line
        let (body_start_line, _) = source.offset_to_line_col(body_node.location().start_offset());
        let body_end_off = body_node
            .location()
            .end_offset()
            .saturating_sub(1)
            .max(body_node.location().start_offset());
        let (body_end_line, _) = source.offset_to_line_col(body_end_off);
        if body_start_line != body_end_line {
            return;
        }

        // Predicate (condition) must be on a single line
        let predicate = if let Some(while_node) = node.as_while_node() {
            while_node.predicate()
        } else if let Some(until_node) = node.as_until_node() {
            until_node.predicate()
        } else {
            return;
        };

        let (pred_start_line, _) = source.offset_to_line_col(predicate.location().start_offset());
        let pred_end_off = predicate
            .location()
            .end_offset()
            .saturating_sub(1)
            .max(predicate.location().start_offset());
        let (pred_end_line, _) = source.offset_to_line_col(pred_end_off);
        if pred_start_line != pred_end_line {
            return;
        }

        // Skip if the condition contains a local variable assignment
        // (e.g., `while (chunk = file.read(1024))`)
        if contains_lvar_assignment(&predicate) {
            return;
        }

        // code_after(node): text after the `end` keyword on the same line
        let closing_end = closing_loc.end_offset();
        let closing_line_end = src_bytes[closing_end..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| closing_end + p)
            .unwrap_or(src_bytes.len());
        let code_after = &src_bytes[closing_end..closing_line_end];
        let code_after_trimmed = code_after
            .iter()
            .skip_while(|&&b| b == b' ' || b == b'\t')
            .copied()
            .collect::<Vec<_>>();
        let has_code_after = !code_after_trimmed.is_empty();

        // first_line_comment(node): comment on the keyword's line (non-disable)
        let (kw_line, _) = source.offset_to_line_col(kw_loc.start_offset());
        let has_first_line_comment = _parse_result.comments().any(|comment| {
            let (comment_line, _) = source.offset_to_line_col(comment.location().start_offset());
            comment_line == kw_line
        });

        // first_line_comment && code_after => skip
        if has_first_line_comment && has_code_after {
            return;
        }

        // Check if the modifier form would fit within the max line length.
        // RuboCop considers Layout/LineLength Max (default 120).
        let max_line_length = _config
            .options
            .get("MaxLineLength")
            .and_then(|v| v.as_u64())
            .unwrap_or(120) as usize;

        // Estimate modifier form length: "body keyword condition"
        let body_src =
            &src_bytes[body_node.location().start_offset()..body_node.location().end_offset()];
        let body_str = String::from_utf8_lossy(body_src);
        let body_trimmed = body_str.trim();

        let pred_src =
            &src_bytes[predicate.location().start_offset()..predicate.location().end_offset()];
        let pred_str = String::from_utf8_lossy(pred_src);

        // Calculate indentation of the original while/until keyword
        let kw_offset = kw_loc.start_offset();
        // Walk back to find the start of the line
        let line_start = src_bytes[..kw_offset]
            .iter()
            .rposition(|&b| b == b'\n')
            .map(|p| p + 1)
            .unwrap_or(0);
        let indent = src_bytes[line_start..]
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();

        // Include first_line_comment in modifier form if present
        let first_line_comment_len = if has_first_line_comment {
            // Find the comment text
            let mut comment_len = 0;
            for comment in _parse_result.comments() {
                let (cl, _) = source.offset_to_line_col(comment.location().start_offset());
                if cl == kw_line {
                    let csrc = &src_bytes
                        [comment.location().start_offset()..comment.location().end_offset()];
                    comment_len = 1 + csrc.len(); // space + comment
                    break;
                }
            }
            comment_len
        } else {
            0
        };

        // Include code_after in modifier length
        let code_after_len = if has_code_after {
            String::from_utf8_lossy(&code_after_trimmed).len()
        } else {
            0
        };

        // "  body keyword condition [# comment] [trailing_code]"
        let modifier_len = indent
            + body_trimmed.len()
            + 1
            + keyword.len()
            + 1
            + pred_str.len()
            + first_line_comment_len
            + code_after_len;
        if modifier_len > max_line_length {
            return;
        }

        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Favor modifier `{}` usage when having a single-line body.",
                keyword
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(WhileUntilModifier, "cops/style/while_until_modifier");
}
