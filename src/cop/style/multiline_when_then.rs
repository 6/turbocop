use crate::cop::node_type::WHEN_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/MultilineWhenThen - flags `then` keyword in multiline `when` clauses.
///
/// ## Corpus investigation (2026-03-18)
/// - FP root cause: conditions spanning multiple lines (e.g., `when 'a',\n 'b' then`)
///   were flagged, but RuboCop allows `then` when conditions span multiple lines
///   because the `then` serves as a visual separator.
/// - FN root cause: `then` on a separate line from `when` (e.g., `when "Work"\n  then ...`)
///   was missed because the old code checked if body was on same line as `then`,
///   but RuboCop checks if body is on same line as the `when` keyword.
/// - Fix: (1) skip when conditions span multiple lines, matching RuboCop's
///   `require_then?` which returns true when first_condition.first_line !=
///   last_condition.last_line; (2) check body vs `when` keyword line, not vs `then` line.
pub struct MultilineWhenThen;

impl Cop for MultilineWhenThen {
    fn name(&self) -> &'static str {
        "Style/MultilineWhenThen"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[WHEN_NODE]
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
        let when_node = match node.as_when_node() {
            Some(w) => w,
            None => return,
        };

        // Check for `then` keyword
        let then_loc = match when_node.then_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        if then_loc.as_slice() != b"then" {
            return;
        }

        // If conditions span multiple lines, `then` is allowed (matches RuboCop's
        // require_then? which returns true when first_line != last_line of conditions).
        let conditions: Vec<_> = when_node.conditions().into_iter().collect();
        if !conditions.is_empty() {
            let first_line = source
                .offset_to_line_col(conditions[0].location().start_offset())
                .0;
            let last_line = source
                .offset_to_line_col(
                    conditions
                        .last()
                        .unwrap()
                        .location()
                        .end_offset()
                        .saturating_sub(1),
                )
                .0;
            if first_line != last_line {
                return;
            }
        }

        // RuboCop's require_then? returns false (offense) when there is no body.
        // When there IS a body, it checks same_line?(when_node, when_node.body) —
        // i.e., whether the `when` keyword and body are on the same line.
        let when_keyword_line = source
            .offset_to_line_col(when_node.keyword_loc().start_offset())
            .0;
        if let Some(stmts) = when_node.statements() {
            let body_nodes: Vec<_> = stmts.body().into_iter().collect();
            if !body_nodes.is_empty() {
                let first_body_line = source
                    .offset_to_line_col(body_nodes[0].location().start_offset())
                    .0;
                if first_body_line == when_keyword_line {
                    // Body is on same line as `when` — single-line style, `then` required.
                    return;
                }
            }
        }

        let (line, column) = source.offset_to_line_col(then_loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            "Do not use `then` for multiline `when` statement.".to_string(),
        );
        // Autocorrect: remove `then` keyword.
        // If at end of line (`when bar then\n`), remove ` then`.
        // If at start of line (`  then do_something\n`), remove `then `.
        if let Some(ref mut corr) = corrections {
            let src = source.as_bytes();
            let then_end = then_loc.end_offset();
            let then_start = then_loc.start_offset();
            // Check if there's a space before `then`
            let remove_start = if then_start > 0 && src[then_start - 1] == b' ' {
                then_start - 1
            } else {
                then_start
            };
            // Check if there's a space after `then`
            let remove_end = if then_end < src.len() && src[then_end] == b' ' {
                then_end + 1
            } else {
                then_end
            };
            // Prefer removing preceding space if at end of line
            let at_eol = then_end >= src.len() || src[then_end] == b'\n' || src[then_end] == b'\r';
            if at_eol {
                corr.push(crate::correction::Correction {
                    start: remove_start,
                    end: then_end,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            } else {
                // `then` followed by content — remove `then ` (keep indentation before)
                corr.push(crate::correction::Correction {
                    start: then_start,
                    end: remove_end,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            }
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineWhenThen, "cops/style/multiline_when_then");
    crate::cop_autocorrect_fixture_tests!(MultilineWhenThen, "cops/style/multiline_when_then");
}
