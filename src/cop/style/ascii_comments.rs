use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/AsciiComments: Use only ASCII symbols in comments.
///
/// Root cause of prior FPs (~1,549): The old `check_lines` approach used
/// `line_str.find('#')` to detect comment starts, which matched `#` inside
/// string literals (interpolation `"#{var}"`, HTML entities `"&#83;"`, etc.).
///
/// A pure Prism approach was tried (commit fc9eb19) but reverted because it
/// produced ~1,090 different excess offenses — likely from Prism including
/// shebang lines, `__END__` sections, or encoding differences vs RuboCop's
/// `processed_source.comments`.
///
/// Current fix (2026-03-08): Uses `check_source` with Prism's `parse_result.comments()`
/// to get accurate comment byte ranges. For each Prism comment, scans only
/// within that byte range for non-ASCII characters. This avoids both the
/// string-literal FPs (old approach) and the shebang/encoding issues (reverted
/// approach) because we now correctly scope scanning to real comment content
/// only, using the same AllowedChars config as before.
pub struct AsciiComments;

impl Cop for AsciiComments {
    fn name(&self) -> &'static str {
        "Style/AsciiComments"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_chars = config.get_string_array("AllowedChars").unwrap_or_default();
        let bytes = source.as_bytes();

        for comment in parse_result.comments() {
            let loc = comment.location();
            let start = loc.start_offset();
            let end = loc.end_offset();

            // Get the comment text (everything from # to end of comment)
            let comment_bytes = &bytes[start..end];
            let comment_text = match std::str::from_utf8(comment_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Skip the leading '#' (and optional '!' for shebangs)
            // RuboCop doesn't flag shebang lines — skip comments starting with #!
            if comment_text.starts_with("#!") {
                continue;
            }

            // Get the text after the '#'
            let after_hash = &comment_text[1..];

            // Find first non-ASCII character in the comment content
            for (char_idx, ch) in after_hash.char_indices() {
                if !ch.is_ascii() {
                    // Check if this character is in the allowed list
                    let ch_str = ch.to_string();
                    if allowed_chars.iter().any(|a| a == &ch_str) {
                        continue;
                    }

                    // Calculate position: offset of '#' + 1 (skip #) + char_idx
                    let byte_offset = start + 1 + char_idx;
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
