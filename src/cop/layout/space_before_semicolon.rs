use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=49, FN=0.
///
/// FP=49 root cause: generated parser code often contains standalone semicolon
/// lines (for example, an indented line containing only `;`). RuboCop ignores
/// indentation before such semicolons, but the previous implementation flagged any
/// ` ;` sequence in code.
///
/// Fix: detect the full whitespace run immediately before `;` and skip offenses
/// when `;` is the first non-whitespace token on the line. Keep offenses for
/// true in-line spacing before semicolons.
///
/// Second fix (2026-03-08): FP=5 all from `{ ; }` pattern — a block open brace
/// with space before semicolon. RuboCop's `SpaceBeforePunctuation` mixin checks
/// `space_required_after?` which returns true when the preceding token is `{` and
/// `Layout/SpaceInsideBlockBraces` has `EnforcedStyle: space` (the default). This
/// defers the space to `SpaceInsideBlockBraces`. Fix: skip offense when the
/// character immediately before the whitespace run is `{`.
///
/// Third fix (2026-03-31): FP=1 in metasm at `when ?\ ; toggle_view(:listing)`.
/// The scanner treated the space inside the escaped-space character literal `?\ `
/// as separator whitespace before the semicolon. RuboCop accepts `?\ ;` but still
/// flags `?\  ;`, so we now consume only the token-internal whitespace byte when
/// the whitespace run starts immediately after `?\`.
pub struct SpaceBeforeSemicolon;

fn escaped_space_char_literal_prefix_len(
    bytes: &[u8],
    whitespace_start: usize,
    semicolon: usize,
) -> usize {
    if whitespace_start < semicolon
        && whitespace_start >= 2
        && bytes[whitespace_start - 2] == b'?'
        && bytes[whitespace_start - 1] == b'\\'
    {
        1
    } else {
        0
    }
}

impl Cop for SpaceBeforeSemicolon {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeSemicolon"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();
        for (i, &byte) in bytes.iter().enumerate() {
            if byte != b';' || i == 0 || !code_map.is_code(i) {
                continue;
            }

            let line_start = bytes[..i]
                .iter()
                .rposition(|&b| b == b'\n')
                .map_or(0, |idx| idx + 1);

            let mut whitespace_start = i;
            while whitespace_start > line_start
                && (bytes[whitespace_start - 1] == b' ' || bytes[whitespace_start - 1] == b'\t')
            {
                whitespace_start -= 1;
            }

            whitespace_start += escaped_space_char_literal_prefix_len(bytes, whitespace_start, i);

            // No space before semicolon.
            if whitespace_start == i {
                continue;
            }

            // Semicolon is the first non-whitespace token on this line.
            if whitespace_start == line_start {
                continue;
            }

            // Space after block open brace is handled by SpaceInsideBlockBraces,
            // not SpaceBeforeSemicolon (e.g., `{ ; }`, `{ ; expr }`).
            if bytes[whitespace_start - 1] == b'{' {
                continue;
            }

            let (line, column) = source.offset_to_line_col(whitespace_start);
            let mut diag = self.diagnostic(
                source,
                line,
                column,
                "Space found before semicolon.".to_string(),
            );
            if let Some(ref mut corr) = corrections {
                corr.push(crate::correction::Correction {
                    start: whitespace_start,
                    end: i,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeSemicolon, "cops/layout/space_before_semicolon");
    crate::cop_autocorrect_fixture_tests!(
        SpaceBeforeSemicolon,
        "cops/layout/space_before_semicolon"
    );

    #[test]
    fn autocorrect_remove_space() {
        let input = b"x = 1 ;\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&SpaceBeforeSemicolon, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1;\n");
    }
}
