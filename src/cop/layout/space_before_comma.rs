use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceBeforeComma;

impl Cop for SpaceBeforeComma {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeComma"
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
            if byte == b',' && i > 0 && bytes[i - 1] == b' ' && code_map.is_code(i) {
                let (line, column) = source.offset_to_line_col(i - 1);
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Space found before comma.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: i - 1,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeComma, "cops/layout/space_before_comma");

    #[test]
    fn autocorrect_remove_space() {
        let input = b"foo(1 , 2)\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceBeforeComma, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"foo(1, 2)\n");
    }

    #[test]
    fn autocorrect_multiple() {
        let input = b"foo(1 , 2 , 3)\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceBeforeComma, input);
        assert_eq!(corrections.len(), 2);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"foo(1, 2, 3)\n");
    }
}
