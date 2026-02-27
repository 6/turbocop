use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct AsciiComments;

impl Cop for AsciiComments {
    fn name(&self) -> &'static str {
        "Style/AsciiComments"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_chars = config.get_string_array("AllowedChars").unwrap_or_default();
        let src_bytes = source.as_bytes();

        for comment in parse_result.comments() {
            let loc = comment.location();
            let comment_bytes = &src_bytes[loc.start_offset()..loc.end_offset()];
            let comment_str = match std::str::from_utf8(comment_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Skip the leading '#' to get just the comment text
            let text = comment_str.strip_prefix('#').unwrap_or(comment_str);
            let text_offset = loc.start_offset() + (comment_str.len() - text.len());

            // Find first non-ASCII character in the comment text
            for (char_idx, ch) in text.char_indices() {
                if !ch.is_ascii() {
                    let ch_str = ch.to_string();
                    if allowed_chars.iter().any(|a| a == &ch_str) {
                        continue;
                    }

                    let byte_offset = text_offset + char_idx;
                    let (line, col) = source.offset_to_line_col(byte_offset);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Use only ascii symbols in comments.".to_string(),
                    ));
                    break; // Only report first non-ASCII per comment
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AsciiComments, "cops/style/ascii_comments");
}
