use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct EndOfLine;

impl Cop for EndOfLine {
    fn name(&self) -> &'static str {
        "Layout/EndOfLine"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for (i, line) in source.lines().enumerate() {
            if line.ends_with(b"\r") {
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location {
                        line: i + 1,
                        column: line.len() - 1,
                    },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Carriage return character detected.".to_string(),
                });
            }
        }
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    #[test]
    fn crlf_detected() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\r\ny = 2\r\n".to_vec());
        let diags = EndOfLine.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 5);
        assert_eq!(diags[0].message, "Carriage return character detected.");
        assert_eq!(diags[1].location.line, 2);
        assert_eq!(diags[1].location.column, 5);
    }

    #[test]
    fn lf_only_no_offense() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\ny = 2\n".to_vec());
        let diags = EndOfLine.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn mixed_line_endings() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\r\ny = 2\n".to_vec());
        let diags = EndOfLine.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
    }

    #[test]
    fn cr_only_at_end() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\r".to_vec());
        let diags = EndOfLine.check_lines(&source, &CopConfig::default());
        // No \n split, so entire content is one line ending with \r
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 5);
    }
}
