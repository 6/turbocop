use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct InitialIndentation;

impl Cop for InitialIndentation {
    fn name(&self) -> &'static str {
        "Layout/InitialIndentation"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        // Find the first non-empty line
        for (i, line) in source.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            if line[0] == b' ' || line[0] == b'\t' {
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: i + 1,
                        column: 0,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Indentation of first line detected.".to_string(),
                }];
            }
            break;
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    #[test]
    fn indented_first_line_space() {
        let source = SourceFile::from_bytes("test.rb", b"  x = 1\n".to_vec());
        let diags = InitialIndentation.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 0);
        assert_eq!(diags[0].message, "Indentation of first line detected.");
    }

    #[test]
    fn indented_first_line_tab() {
        let source = SourceFile::from_bytes("test.rb", b"\tx = 1\n".to_vec());
        let diags = InitialIndentation.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
    }

    #[test]
    fn no_indentation() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n".to_vec());
        let diags = InitialIndentation.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn leading_blank_then_indented() {
        let source = SourceFile::from_bytes("test.rb", b"\n  x = 1\n".to_vec());
        let diags = InitialIndentation.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
    }

    #[test]
    fn leading_blank_then_unindented() {
        let source = SourceFile::from_bytes("test.rb", b"\nx = 1\n".to_vec());
        let diags = InitialIndentation.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn empty_file() {
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let diags = InitialIndentation.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }
}
