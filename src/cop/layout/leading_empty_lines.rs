use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct LeadingEmptyLines;

impl Cop for LeadingEmptyLines {
    fn name(&self) -> &'static str {
        "Layout/LeadingEmptyLines"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let bytes = source.as_bytes();
        if bytes.is_empty() {
            return Vec::new();
        }

        if bytes[0] == b'\n' {
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line: 1, column: 0 },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Unnecessary blank line at the beginning of the source.".to_string(),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses, assert_cop_offenses};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses(
            &LeadingEmptyLines,
            include_bytes!("../../../testdata/cops/layout/leading_empty_lines/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses(
            &LeadingEmptyLines,
            include_bytes!("../../../testdata/cops/layout/leading_empty_lines/no_offense.rb"),
        );
    }

    #[test]
    fn multiple_leading_blank_lines() {
        let source = SourceFile::from_bytes("test.rb", b"\n\nx = 1\n".to_vec());
        let diags = LeadingEmptyLines.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
    }

    #[test]
    fn empty_file() {
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let diags = LeadingEmptyLines.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }
}
