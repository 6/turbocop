use crate::cop::node_type::WHEN_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus: 5 FPs fixed.
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
pub struct EmptyWhen;

impl Cop for EmptyWhen {
    fn name(&self) -> &'static str {
        "Lint/EmptyWhen"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[WHEN_NODE]
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
        let when_node = match node.as_when_node() {
            Some(n) => n,
            None => return,
        };

        let body_empty = match when_node.statements() {
            None => true,
            Some(stmts) => stmts.body().is_empty(),
        };

        if !body_empty {
            return;
        }

        // AllowComments: when true, `when` bodies containing only comments are not offenses
        let allow_comments = config.get_bool("AllowComments", true);
        if allow_comments {
            // The WhenNode's location only covers the `when` keyword + conditions,
            // NOT inline comments (e.g., `when "C" ; # comment`) or standalone
            // comment lines below an empty when body. Extend the search range from
            // the when keyword through all subsequent blank/comment lines until the
            // next code token (next when/else/end).
            let when_start = when_node.keyword_loc().start_offset();
            let src = source.as_bytes();

            // For multi-line when conditions, start scanning from after the
            // last condition expression, not from the `when` keyword line.
            // For heredoc conditions (StringNode, InterpolatedStringNode), the
            // node's location only covers the opening delimiter (e.g. `<<~TEXT`),
            // but the actual content and closing delimiter extend much further.
            // Use closing_loc when available to get the true end offset.
            let conditions = when_node.conditions();
            let last_cond_end = conditions
                .iter()
                .map(|c| {
                    let loc_end = c.location().end_offset();
                    // Check for heredoc closing delimiter beyond the node location
                    let closing_end = if let Some(s) = c.as_string_node() {
                        s.closing_loc().map(|l| l.end_offset()).unwrap_or(loc_end)
                    } else if let Some(s) = c.as_interpolated_string_node() {
                        s.closing_loc().map(|l| l.end_offset()).unwrap_or(loc_end)
                    } else {
                        loc_end
                    };
                    loc_end.max(closing_end)
                })
                .max()
                .unwrap_or(when_start);

            // Find the end of the line containing the last condition
            let line_end = src[last_cond_end..]
                .iter()
                .position(|&b| b == b'\n')
                .map_or(src.len(), |p| last_cond_end + p);

            // Extend past subsequent blank/comment-only lines
            let mut search_end = line_end;
            let mut pos = if line_end < src.len() {
                line_end + 1
            } else {
                src.len()
            };
            while pos < src.len() {
                let next_nl = src[pos..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map_or(src.len(), |p| pos + p);
                let line = &src[pos..next_nl];
                let trimmed = line
                    .iter()
                    .skip_while(|b| b.is_ascii_whitespace())
                    .copied()
                    .collect::<Vec<u8>>();
                if trimmed.is_empty() || trimmed.starts_with(b"#") {
                    search_end = next_nl;
                    pos = if next_nl < src.len() {
                        next_nl + 1
                    } else {
                        src.len()
                    };
                } else {
                    break;
                }
            }

            for comment in _parse_result.comments() {
                let comment_start = comment.location().start_offset();
                if comment_start >= when_start && comment_start <= search_end {
                    return;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyWhen, "cops/lint/empty_when");
}
