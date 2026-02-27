use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct AsciiComments;

// KNOWN ISSUE (2026-02-27): This cop has ~1,043 FPs from the naive `find('#')`
// below — it treats `#` inside string literals (interpolation, HTML entities like
// "&#83;") as comment starts. We attempted switching to `check_source` with
// `parse_result.comments()` (Prism's actual comment nodes), which eliminated the
// string FPs entirely. However, the Prism-based approach produced ~1,090 DIFFERENT
// excess offenses on real comments that RuboCop doesn't flag. Possible causes:
//   1. Prism returns more comment nodes than Parser gem (e.g., shebang, __END__)
//   2. RuboCop's `processed_source.comments` filters certain comment types
//   3. Encoding-related differences in what's considered "non-ASCII"
// The Prism approach is in git history (commit fc9eb19, reverted). To fix properly,
// need to understand why RuboCop reports fewer non-ASCII comment offenses — run
// RuboCop directly on a high-excess repo (e.g., jruby, stripe-ruby) and compare
// offense locations with nitrocop's Prism-based output.

impl Cop for AsciiComments {
    fn name(&self) -> &'static str {
        "Style/AsciiComments"
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_chars = config.get_string_array("AllowedChars").unwrap_or_default();

        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Find comment portion of the line.
            // BUG: This finds the first `#` on the line, which may be inside a
            // string literal (e.g., "#{var}", "&#83;"), causing false positives
            // on non-ASCII characters in strings. See comment above for details.
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
