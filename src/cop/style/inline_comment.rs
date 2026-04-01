use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Style/InlineComment: Avoid trailing inline comments.
///
/// RuboCop flags embedded documentation comments (`=begin`/`=end`) as
/// `Style/InlineComment` offenses because `comment_line?` only treats lines
/// starting with `#` as standalone comments. Prism exposes those as
/// `EmbDocComment`, not `InlineComment`, so this cop must not apply the
/// standalone-`#` shortcut to them.
///
/// RuboCop only exempts exact inline directives like `# rubocop:disable Foo`
/// and `# rubocop:enable Foo`. Variants such as `#rubocop:disable Foo`,
/// `#  rubocop:disable Foo`, and `# rubocop: disable Foo` are still offenses.
///
/// Prism also reports `#` tokens inside string bodies and inside multiline
/// interpolation openers (`#{# comment`) as comments. RuboCop does not treat
/// those as trailing inline comments, so this cop must skip comment offsets that
/// fall inside string content and comment lines whose only code prefix is `#{`.
///
/// Comments inside heredoc interpolation (`<<~H\n  text #{expr # comment}\nH`)
/// are real code comments even though the CodeMap marks the entire heredoc body
/// as "string". The cop uses `is_heredoc_interpolation` to detect these.
///
/// RuboCop's `comment_line?` uses `/^\s*#/` on the full source line, so lines
/// starting with `#` from `#{` interpolation syntax are treated as comment
/// lines. The cop matches this by skipping comments whose line prefix (trimmed)
/// starts with `#`.
pub struct InlineComment;

impl Cop for InlineComment {
    fn name(&self) -> &'static str {
        "Style/InlineComment"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();

        for comment in parse_result.comments() {
            let is_embdoc = matches!(comment.type_(), ruby_prism::CommentType::EmbDocComment);
            let loc = comment.location();
            let start = loc.start_offset();

            if !is_embdoc {
                // Prism still reports `#` inside string literal bodies as comments,
                // but RuboCop's processed comments exclude those from this cop.
                // Exception: comments inside heredoc interpolation (`#{...}`) ARE
                // real code comments — only skip if the position is truly in string
                // content, not in interpolation code.
                if !code_map.is_not_string(start)
                    && (!code_map.is_heredoc_interpolation(start)
                        || code_map.is_non_code_in_heredoc_interpolation(start))
                {
                    continue;
                }

                // Skip if this is the first character in the file
                if start == 0 {
                    continue;
                }

                // Find the start of the current line
                let mut line_start = start;
                while line_start > 0 && bytes[line_start - 1] != b'\n' {
                    line_start -= 1;
                }

                // Get content before the comment on this line
                let before_on_line = &bytes[line_start..start];
                let trimmed_before = trim_ascii_whitespace(before_on_line);

                // If only whitespace before the comment, it's a standalone `#` comment
                if trimmed_before.is_empty() {
                    continue;
                }

                // RuboCop's `comment_line?` checks `/^\s*#/` on the full source
                // line. If the line starts with `#` (after whitespace), RuboCop
                // treats it as a comment line even when the `#` is from `#{`
                // interpolation syntax, not an actual comment.
                if trimmed_before.starts_with(b"#") {
                    continue;
                }

                // This is an inline `#` comment — check for rubocop/nitrocop directives
                let comment_bytes = &bytes[start..loc.end_offset()];
                let comment_text = match std::str::from_utf8(comment_bytes) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if is_exempt_inline_directive(comment_text) {
                    continue;
                }
            }

            let (line, col) = source.offset_to_line_col(start);
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Avoid trailing inline comments.".to_string(),
            ));
        }
    }
}

fn trim_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map(|idx| idx + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

fn is_exempt_inline_directive(comment_text: &str) -> bool {
    comment_text.starts_with("# rubocop:enable")
        || comment_text.starts_with("# rubocop:disable")
        || comment_text.starts_with("# nitrocop:enable")
        || comment_text.starts_with("# nitrocop:disable")
        || comment_text.starts_with("# nitrocop:todo")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InlineComment, "cops/style/inline_comment");
}
