use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceAfterComma;

impl Cop for SpaceAfterComma {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterComma"
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
            if byte == b',' && code_map.is_code(i) {
                // Skip if this comma is part of a global variable ($, or $;)
                if i > 0 && bytes[i - 1] == b'$' {
                    continue;
                }
                let next = bytes.get(i + 1).copied();
                // Skip commas before closing delimiters — RuboCop's
                // SpaceAfterPunctuation#allowed_type? skips ), ], and |.
                if matches!(next, Some(b')') | Some(b']') | Some(b'|')) {
                    continue;
                }
                if !matches!(next, Some(b' ') | Some(b'\n') | Some(b'\r') | None) {
                    let (line, column) = source.offset_to_line_col(i);
                    let mut diag = self.diagnostic(
                        source,
                        line,
                        column,
                        "Space missing after comma.".to_string(),
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

    crate::cop_fixture_tests!(SpaceAfterComma, "cops/layout/space_after_comma");

    #[test]
    fn autocorrect_insert_space() {
        let input = b"foo(1,2)\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceAfterComma, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"foo(1, 2)\n");
    }

    #[test]
    fn autocorrect_multiple() {
        let input = b"foo(1,2,3)\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceAfterComma, input);
        assert_eq!(corrections.len(), 2);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"foo(1, 2, 3)\n");
    }
}
