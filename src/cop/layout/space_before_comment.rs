use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=8, FN=0.
///
/// FP=8 root cause: files beginning with a UTF-8 BOM followed by a standalone
/// magic comment were treated as inline comments because the BOM bytes appeared
/// before `#` on line 1. RuboCop tokenizes those files as a leading comment and
/// does not require a separating space.
///
/// Fix: treat a line-prefix UTF-8 BOM like leading whitespace when deciding
/// whether a comment starts the line.
///
/// Rerun outcome: removed the CI-baseline false positives from `ifme` (5),
/// `dryrun` (1), and one `natalie` case. Local reruns still show one legacy
/// `jruby` false positive plus offenses in an excluded local-only corpus repo.
///
/// ## FN investigation (2026-03-21)
///
/// FN=4 root cause: In Ruby, `?\ ` (character literal with backslash-space) is
/// recognized as a space escape sequence, producing a space character. The space
/// is "consumed" by the escape, meaning there's no whitespace token between the
/// character literal and the following comment. RuboCop correctly detects this
/// because its position-based check (`token1.pos.end == token2.pos.begin`) sees
/// the character literal ending where the comment begins.
///
/// The Rust implementation checked `bytes[start - 1]` for a space character, which
/// would be the space from `?\ `. It then skipped the offense. But that space is
/// not a "separator" - it's part of the character literal's output.
///
/// Fix: detect when the space before a comment is from `?\ ` by checking if
/// bytes[start-3..start] == `?\ ` (question mark, backslash, space). In that
/// case, the space is from the character literal escape and should not be
/// treated as a separator. Report the offense.
pub struct SpaceBeforeComment;

impl Cop for SpaceBeforeComment {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeComment"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();

        for comment in parse_result.comments() {
            let loc = comment.location();
            let start = loc.start_offset();

            // Skip if this is the first character on the line (standalone comment)
            if start == 0 {
                continue;
            }
            let prev = bytes[start - 1];
            if prev == b'\n' || prev == b'\r' {
                continue;
            }
            // Skip if preceded only by whitespace (indented standalone comment)
            let mut line_start = start;
            while line_start > 0 && bytes[line_start - 1] != b'\n' {
                line_start -= 1;
            }
            let before_on_line = &bytes[line_start..start];
            let before_on_line = before_on_line
                .strip_prefix(b"\xEF\xBB\xBF")
                .unwrap_or(before_on_line);
            if before_on_line.iter().all(|&b| b == b' ' || b == b'\t') {
                continue;
            }

            // Inline comment: check for space before #
            // In Ruby, `\ ` (backslash-space) is recognized as a space escape in character
            // literals (e.g., `?\ ` produces a space character). The space is consumed by the
            // escape, so there's no separator space between the character literal and the comment.
            // Detect this: `?\ ` is `?` (start-3), `\` (start-2), ` ` (start-1).
            let is_space_from_char_escape = start >= 3
                && bytes[start - 2] == b'\\'
                && bytes[start - 3] == b'?';
            if prev != b' ' && prev != b'\t' || (prev == b' ' && is_space_from_char_escape) {
                let (line, column) = source.offset_to_line_col(start);
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Put a space before an end-of-line comment.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start,
                        end: start,
                        replacement: " ".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeComment, "cops/layout/space_before_comment");
    crate::cop_autocorrect_fixture_tests!(SpaceBeforeComment, "cops/layout/space_before_comment");

    #[test]
    fn autocorrect_insert_space() {
        let input = b"x = 1# comment\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&SpaceBeforeComment, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1 # comment\n");
    }
}
