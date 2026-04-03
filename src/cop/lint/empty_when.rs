use crate::cop::shared::node_type::CASE_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus: 6 FPs fixed, 22 FNs fixed.
///
/// Round 1 (4 FPs): Multi-line `when` conditions (conditions spanning multiple lines)
/// with comment-only bodies. The AllowComments search started from the `when` keyword
/// line, so for multi-line conditions the continuation lines (containing code) broke
/// the blank/comment scan before reaching the actual comment.
/// Fix: start the scan from the end of the last condition expression instead.
///
/// Round 2 (1 FP): Heredoc condition (`when <<~TEXT ... TEXT`). Prism's StringNode
/// location only covers the opening delimiter (`<<~TEXT`), not the heredoc body or
/// closing delimiter. The line-forward scan from `last_cond_end` hit the heredoc
/// content lines (not blank, not comment) and stopped before reaching the actual
/// comment in the when body. Fix: use `closing_loc().end_offset()` for StringNode
/// and InterpolatedStringNode conditions to get the true end past the heredoc.
///
/// Round 3 (1 FP): Empty `when` before `else` with comment in else body. Pattern:
/// `when /\Afile:/\nelse\n  # comment\n  code`. RuboCop's CommentsHelp#find_end_line
/// uses the right_sibling's start line as the search boundary, which extends past
/// the `else` keyword into the else body. Comments between `else` and the first
/// else-body statement are found, suppressing the offense.
///
/// Round 4 (22 FNs): The line-scanning approach for AllowComments extended through
/// `when`/`else`/`end` keyword lines, causing the search to escape the current when
/// branch's scope. Comments in later when bodies or after the case `end` keyword
/// would incorrectly suppress offenses for earlier empty when branches.
/// Fix: switched from WHEN_NODE to CASE_NODE processing, iterating over when branches
/// with AST-based boundary computation. For each empty when, the comment search range
/// is bounded by the next when's start offset, the else body's first statement offset,
/// or the case's end keyword offset — matching RuboCop's CommentsHelp#find_end_line.
pub struct EmptyWhen;

impl Cop for EmptyWhen {
    fn name(&self) -> &'static str {
        "Lint/EmptyWhen"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CASE_NODE]
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
        let case_node = match node.as_case_node() {
            Some(n) => n,
            None => return,
        };

        let allow_comments = config.get_bool("AllowComments", true);
        let whens: Vec<_> = case_node.conditions().iter().collect();

        for (i, when_ref) in whens.iter().enumerate() {
            let when_node = match when_ref.as_when_node() {
                Some(w) => w,
                None => continue,
            };

            let body_empty = match when_node.statements() {
                None => true,
                Some(stmts) => stmts.body().is_empty(),
            };

            if !body_empty {
                continue;
            }

            if allow_comments {
                let when_start = when_node.keyword_loc().start_offset();

                // Compute the search boundary for comments, matching RuboCop's
                // CommentsHelp#find_end_line which uses:
                // 1. The next sibling when's start offset
                // 2. The else body's first statement offset (extends past `else` keyword)
                // 3. The case's end keyword offset
                let search_end = if i + 1 < whens.len() {
                    whens[i + 1].location().start_offset()
                } else if let Some(else_clause) = case_node.else_clause() {
                    // RuboCop's find_end_line returns the right_sibling's start line.
                    // In Parser's AST, the else body starts at the first statement,
                    // so comments between `else` and the first statement are in range.
                    else_clause
                        .statements()
                        .and_then(|stmts| stmts.body().iter().next())
                        .map(|first_stmt| first_stmt.location().start_offset())
                        .unwrap_or_else(|| case_node.end_keyword_loc().start_offset())
                } else {
                    case_node.end_keyword_loc().start_offset()
                };

                // Check for any comment in the range [when_start, search_end).
                // This covers:
                // - Inline comments on the when line (e.g., `when :foo ; # comment`)
                // - Standalone comment lines in the body
                // - `then # comment` patterns
                let has_comment = _parse_result.comments().any(|comment| {
                    let cs = comment.location().start_offset();
                    cs >= when_start && cs < search_end
                });

                if has_comment {
                    continue;
                }
            }

            let kw_loc = when_node.keyword_loc();
            let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Avoid empty `when` conditions.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyWhen, "cops/lint/empty_when");
}
