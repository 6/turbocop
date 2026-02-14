use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct Semicolon;

impl Cop for Semicolon {
    fn name(&self) -> &'static str {
        "Style/Semicolon"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let bytes = source.as_bytes();
        let mut diagnostics = Vec::new();

        for (i, &byte) in bytes.iter().enumerate() {
            if byte == b';' && code_map.is_code(i) {
                let (line, column) = source.offset_to_line_col(i);
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Do not use semicolons to terminate expressions.".to_string(),
                });
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &Semicolon,
            include_bytes!("../../../testdata/cops/style/semicolon/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &Semicolon,
            include_bytes!("../../../testdata/cops/style/semicolon/no_offense.rb"),
        );
    }
}
