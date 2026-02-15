use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingEmptyLines;

impl Cop for TrailingEmptyLines {
    fn name(&self) -> &'static str {
        "Layout/TrailingEmptyLines"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "final_newline");
        let bytes = source.as_bytes();
        if bytes.is_empty() {
            return Vec::new();
        }

        match style {
            "final_blank_line" => {
                // Require file to end with \n\n (blank line before EOF)
                if *bytes.last().unwrap() != b'\n' {
                    let line_count = bytes.iter().filter(|&&b| b == b'\n').count() + 1;
                    return vec![self.diagnostic(
                        source,
                        line_count,
                        0,
                        "Final newline missing.".to_string(),
                    )];
                }
                // Need at least \n\n at end
                if bytes.len() < 2 || bytes[bytes.len() - 2] != b'\n' {
                    let line_count = bytes.iter().filter(|&&b| b == b'\n').count();
                    return vec![self.diagnostic(
                        source,
                        line_count,
                        0,
                        "Trailing blank line missing.".to_string(),
                    )];
                }
                Vec::new()
            }
            _ => {
                // "final_newline" (default): require exactly one trailing newline
                if *bytes.last().unwrap() != b'\n' {
                    let line_count = bytes.iter().filter(|&&b| b == b'\n').count() + 1;
                    return vec![self.diagnostic(
                        source,
                        line_count,
                        0,
                        "Final newline missing.".to_string(),
                    )];
                }

                // Check for trailing blank lines (content ends with \n\n)
                if bytes.len() >= 2 && bytes[bytes.len() - 2] == b'\n' {
                    let mut end = bytes.len() - 1;
                    while end > 0 && bytes[end - 1] == b'\n' {
                        end -= 1;
                    }
                    let line_num = bytes[..end].iter().filter(|&&b| b == b'\n').count() + 2;
                    return vec![self.diagnostic(
                        source,
                        line_num,
                        0,
                        "Trailing blank line detected.".to_string(),
                    )];
                }

                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    crate::cop_scenario_fixture_tests!(
        TrailingEmptyLines, "cops/layout/trailing_empty_lines",
        missing_newline = "missing_newline.rb",
        trailing_blank = "trailing_blank.rb",
        multiple_trailing = "multiple_trailing.rb",
    );

    #[test]
    fn missing_final_newline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].message, "Final newline missing.");
    }

    #[test]
    fn missing_final_newline_multiline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\ny = 2".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Final newline missing.");
    }

    #[test]
    fn trailing_blank_line() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n\n".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Trailing blank line detected.");
    }

    #[test]
    fn multiple_trailing_blank_lines() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n\n\n\n".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Trailing blank line detected.");
    }

    #[test]
    fn proper_final_newline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn multiline_proper() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\ny = 2\n".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn empty_file() {
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &CopConfig::default());
        assert!(diags.is_empty());
    }

    #[test]
    fn final_blank_line_style_accepts_trailing_blank() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("final_blank_line".into())),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n\n".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &config);
        assert!(diags.is_empty(), "final_blank_line style should accept trailing blank line");
    }

    #[test]
    fn final_blank_line_style_flags_missing_blank() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("final_blank_line".into())),
            ]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n".to_vec());
        let diags = TrailingEmptyLines.check_lines(&source, &config);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "Trailing blank line missing.");
    }
}
