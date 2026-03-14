use crate::cop::node_type::WHEN_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus: 6 FPs fixed.
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
/// else-body statement are found, suppressing the offense. Fix: extend the
/// blank/comment line scan to also skip `when`/`else`/`end` keyword lines.
/// Check if a trimmed line starts with a Ruby case/when structural keyword
/// (`when`, `else`, `end`). Used to extend the AllowComments search range
/// past these keywords to match RuboCop's CommentsHelp behavior.
fn is_ruby_keyword_line(trimmed: &[u8]) -> bool {
    for keyword in &[b"when" as &[u8], b"else", b"end"] {
        if trimmed.starts_with(keyword) {
            // Keyword must be the whole token: followed by whitespace, newline, or end of content
            let rest = &trimmed[keyword.len()..];
            if rest.is_empty()
                || rest[0].is_ascii_whitespace()
                || rest[0] == b'#'
                || rest[0] == b';'
            {
                return true;
            }
        }
    }
    false
}

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

            // Extend past subsequent blank/comment-only lines.
            // Also scan past `when`/`else`/`end` keyword lines to match RuboCop's
            // CommentsHelp#find_end_line, which uses the right_sibling's start line
            // as the end boundary. This means comments in the `else` body (between
            // the `else` keyword and the first statement) are included in the
            // comment search for the preceding empty `when`.
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
                let is_keyword_line = is_ruby_keyword_line(&trimmed);
                if trimmed.is_empty() || trimmed.starts_with(b"#") || is_keyword_line {
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
