use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceBeforeSemicolon;

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
            if byte == b';' && i > 0 && bytes[i - 1] == b' ' && code_map.is_code(i) {
                let (line, column) = source.offset_to_line_col(i - 1);
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Space found before semicolon.".to_string(),
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

    crate::cop_fixture_tests!(SpaceBeforeSemicolon, "cops/layout/space_before_semicolon");
    crate::cop_autocorrect_fixture_tests!(SpaceBeforeSemicolon, "cops/layout/space_before_semicolon");

    #[test]
    fn autocorrect_remove_space() {
        let input = b"x = 1 ;\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceBeforeSemicolon, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1;\n");
    }
}
