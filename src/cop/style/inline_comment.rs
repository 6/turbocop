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
        _code_map: &CodeMap,
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

                // If only whitespace before the comment, it's a standalone `#` comment
                if before_on_line.iter().all(|&b| b == b' ' || b == b'\t') {
                    continue;
                }

                // This is an inline `#` comment — check for rubocop/nitrocop directives
                let comment_bytes = &bytes[start..loc.end_offset()];
                let comment_text = match std::str::from_utf8(comment_bytes) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let after_hash = comment_text.trim_start_matches('#').trim_start();
                if after_hash.starts_with("rubocop:") || after_hash.starts_with("nitrocop-") {
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InlineComment, "cops/style/inline_comment");
}
