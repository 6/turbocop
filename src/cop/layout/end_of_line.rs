use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EndOfLine;

impl Cop for EndOfLine {
    fn name(&self) -> &'static str {
        "Layout/EndOfLine"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, _corrections: Option<&mut Vec<crate::correction::Correction>>) {
        let style = config.get_str("EnforcedStyle", "native");

        match style {
            "lf" | "native" => {
                // Flag lines ending with \r (i.e., CRLF or bare CR)
                for (i, line) in source.lines().enumerate() {
                    if line.ends_with(b"\r") {
                        diagnostics.push(self.diagnostic(
                            source,
                            i + 1,
                            line.len() - 1,
                            "Carriage return character detected.".to_string(),
                        ));
                    }
                }
            }
            "crlf" => {
                // Flag lines that do NOT end with \r (i.e., bare LF)
                // Only check lines that actually have content (skip last empty segment)
                let lines: Vec<&[u8]> = source.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    // Skip the last "line" if it's empty (artifact of trailing \n split)
                    if i == lines.len() - 1 && line.is_empty() {
                        continue;
                    }
                    if !line.ends_with(b"\r") {
                        diagnostics.push(self.diagnostic(
                            source,
                            i + 1,
                            line.len(),
                            "Carriage return character missing.".to_string(),
                        ));
                    }
                }
            }
            _ => {
                // Unknown style, fall back to native (LF) behavior
                for (i, line) in source.lines().enumerate() {
                    if line.ends_with(b"\r") {
                        diagnostics.push(self.diagnostic(
                            source,
                            i + 1,
                            line.len() - 1,
                            "Carriage return character detected.".to_string(),
                        ));
                    }
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    crate::cop_fixture_tests!(EndOfLine, "cops/layout/end_of_line");

    #[test]
    fn crlf_detected() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\r\ny = 2\r\n".to_vec());
        let mut diags = Vec::new();
        EndOfLine.check_lines(&source, &CopConfig::default(), &mut diags, None);
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
        let mut diags = Vec::new();
        EndOfLine.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn mixed_line_endings() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\r\ny = 2\n".to_vec());
        let mut diags = Vec::new();
        EndOfLine.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
    }

    #[test]
    fn cr_only_at_end() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\r".to_vec());
        let mut diags = Vec::new();
        EndOfLine.check_lines(&source, &CopConfig::default(), &mut diags, None);
        // No \n split, so entire content is one line ending with \r
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 5);
    }

    #[test]
    fn crlf_style_accepts_crlf() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("crlf".into())),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"x = 1\r\ny = 2\r\n".to_vec());
        let mut diags = Vec::new();
        EndOfLine.check_lines(&source, &config, &mut diags, None);
        assert!(diags.is_empty(), "crlf style should accept CRLF line endings");
    }

    #[test]
    fn crlf_style_flags_lf() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("crlf".into())),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"x = 1\ny = 2\n".to_vec());
        let mut diags = Vec::new();
        EndOfLine.check_lines(&source, &config, &mut diags, None);
        assert_eq!(diags.len(), 2, "crlf style should flag LF-only lines");
        assert_eq!(diags[0].message, "Carriage return character missing.");
    }
}
