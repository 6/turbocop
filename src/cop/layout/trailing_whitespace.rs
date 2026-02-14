use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct TrailingWhitespace;

impl Cop for TrailingWhitespace {
    fn name(&self) -> &'static str {
        "Layout/TrailingWhitespace"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for (i, line) in source.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let last_content = line.iter().rposition(|&b| b != b' ' && b != b'\t');
            match last_content {
                Some(pos) if pos + 1 < line.len() => {
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location {
                            line: i + 1,
                            column: pos + 1,
                        },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Trailing whitespace detected.".to_string(),
                    });
                }
                None => {
                    // Entire line is whitespace
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location {
                            line: i + 1,
                            column: 0,
                        },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Trailing whitespace detected.".to_string(),
                    });
                }
                _ => {}
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
            &TrailingWhitespace,
            include_bytes!("../../../testdata/cops/layout/trailing_whitespace/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses(
            &TrailingWhitespace,
            include_bytes!("../../../testdata/cops/layout/trailing_whitespace/no_offense.rb"),
        );
    }

    #[test]
    fn all_whitespace_line() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n   \ny = 2\n".to_vec());
        let diags = TrailingWhitespace.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].location.column, 0);
    }

    #[test]
    fn trailing_tab() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\t\n".to_vec());
        let diags = TrailingWhitespace.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 5);
    }

    #[test]
    fn no_trailing_newline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1  ".to_vec());
        let diags = TrailingWhitespace.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 5);
    }
}
