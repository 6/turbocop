use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct Tab;

impl Cop for Tab {
    fn name(&self) -> &'static str {
        "Style/Tab"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for (i, line) in source.lines().enumerate() {
            if let Some(pos) = line.iter().position(|&b| b == b'\t') {
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: i + 1,
                        column: pos,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Tab detected in indentation.".to_string(),
                });
            }
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses, assert_cop_offenses};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses(
            &Tab,
            include_bytes!("../../../testdata/cops/style/tab/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses(
            &Tab,
            include_bytes!("../../../testdata/cops/style/tab/no_offense.rb"),
        );
    }

    #[test]
    fn tab_at_start() {
        let source = SourceFile::from_bytes("test.rb", b"\tx = 1\n".to_vec());
        let diags = Tab.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 0);
    }

    #[test]
    fn tab_in_middle() {
        let source = SourceFile::from_bytes("test.rb", b"x =\t1\n".to_vec());
        let diags = Tab.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.column, 3);
    }

    #[test]
    fn no_tabs() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n  y = 2\n".to_vec());
        let diags = Tab.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn multiple_lines_with_tabs() {
        let source = SourceFile::from_bytes("test.rb", b"\tx = 1\n\ty = 2\n".to_vec());
        let diags = Tab.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[1].location.line, 2);
    }
}
