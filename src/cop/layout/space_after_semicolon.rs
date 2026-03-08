use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=4, FN=0.
///
/// FP=4 root cause: the byte scanner treated semicolons inside the legacy global
/// variable `$;` as statement separators. It also diverged from RuboCop's token
/// handling for consecutive semicolons (`foo;; bar`).
///
/// Fix: skip semicolons that are part of `$;`, ignore semicolon sequences, and
/// accept tabs as whitespace after a statement separator.
///
/// Rerun outcome: removed the original tab-after-semicolon false positive from
/// `ging` plus one baseline FP each in `natalie` and `facets`. Remaining local
/// noise is dominated by `jruby` file drops and one generated `.rbnext` file in
/// `ruby-next` that CI did not count.
pub struct SpaceAfterSemicolon;

impl Cop for SpaceAfterSemicolon {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterSemicolon"
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
            if byte == b';' && code_map.is_code(i) {
                if matches!(bytes.get(i.wrapping_sub(1)), Some(b'$')) {
                    continue;
                }

                let next = bytes.get(i + 1).copied();
                if matches!(next, Some(b';')) {
                    continue;
                }

                if !matches!(
                    next,
                    Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') | None
                ) {
                    let (line, column) = source.offset_to_line_col(i);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space missing after semicolon.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: i + 1,
                            end: i + 1,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceAfterSemicolon, "cops/layout/space_after_semicolon");
    crate::cop_autocorrect_fixture_tests!(SpaceAfterSemicolon, "cops/layout/space_after_semicolon");

    #[test]
    fn autocorrect_insert_space() {
        let input = b"x = 1;y = 2\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&SpaceAfterSemicolon, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1; y = 2\n");
    }
}
