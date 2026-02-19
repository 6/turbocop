use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AsciiComments;

impl Cop for AsciiComments {
    fn name(&self) -> &'static str {
        "Style/AsciiComments"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig, diagnostics: &mut Vec<Diagnostic>) {
        let allowed_chars = config
            .get_string_array("AllowedChars")
            .unwrap_or_default();


        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Find comment portion of the line
            let comment_start = match line_str.find('#') {
                Some(pos) => pos,
                None => continue,
            };

            let comment = &line_str[comment_start + 1..];

            // Find first non-ASCII character in comment
            for (char_idx, ch) in comment.char_indices() {
                if !ch.is_ascii() {
                    // Check if this character is in the allowed list
                    let ch_str = ch.to_string();
                    if allowed_chars.iter().any(|a| a == &ch_str) {
                        continue;
                    }

                    let col = comment_start + 1 + char_idx;
                    diagnostics.push(self.diagnostic(
                        source,
                        i + 1,
                        col,
                        "Use only ascii symbols in comments.".to_string(),
                    ));
                    break; // Only report first non-ASCII per line
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
