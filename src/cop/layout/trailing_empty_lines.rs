use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingEmptyLines;

impl Cop for TrailingEmptyLines {
    fn name(&self) -> &'static str {
        "Layout/TrailingEmptyLines"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "final_newline");
        let bytes = source.as_bytes();
        if bytes.is_empty() {
            return;
        }

        match style {
            "final_blank_line" => {
                // Require file to end with \n\n (blank line before EOF)
                if *bytes.last().unwrap() != b'\n' {
                    let line_count = bytes.iter().filter(|&&b| b == b'\n').count() + 1;
                    let mut diag = self.diagnostic(
                        source,
                        line_count,
                        0,
                        "Final newline missing.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: bytes.len(),
                            end: bytes.len(),
                            replacement: "\n".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                // Need at least \n\n at end
                if bytes.len() < 2 || bytes[bytes.len() - 2] != b'\n' {
                    let line_count = bytes.iter().filter(|&&b| b == b'\n').count();
                    let mut diag = self.diagnostic(
                        source,
                        line_count,
                        0,
                        "Trailing blank line missing.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: bytes.len(),
                            end: bytes.len(),
                            replacement: "\n".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }
            _ => {
                // "final_newline" (default): require exactly one trailing newline
                if *bytes.last().unwrap() != b'\n' {
                    let line_count = bytes.iter().filter(|&&b| b == b'\n').count() + 1;
                    let mut diag = self.diagnostic(
                        source,
                        line_count,
                        0,
                        "Final newline missing.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: bytes.len(),
                            end: bytes.len(),
                            replacement: "\n".to_string(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }

                // Check for trailing blank lines (content ends with \n\n)
                if bytes.len() >= 2 && bytes[bytes.len() - 2] == b'\n' {
                    let mut end = bytes.len() - 1;
                    while end > 0 && bytes[end - 1] == b'\n' {
                        end -= 1;
                    }
                    let line_num = bytes[..end].iter().filter(|&&b| b == b'\n').count() + 2;
                    let mut diag = self.diagnostic(
                        source,
                        line_num,
                        0,
                        "Trailing blank line detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        // Delete from first extra \n to end, keeping exactly one \n
                        // end points to the byte after the last content newline
                        corr.push(crate::correction::Correction {
                            start: end + 1,
                            end: bytes.len(),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    crate::cop_scenario_fixture_tests!(
        TrailingEmptyLines,
        "cops/layout/trailing_empty_lines",
        missing_newline = "missing_newline.rb",
        trailing_blank = "trailing_blank.rb",
        multiple_trailing = "multiple_trailing.rb",
    );

    #[test]
    fn missing_final_newline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].message, "Final newline missing.");
    }

    #[test]
    fn missing_final_newline_multiline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\ny = 2".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Final newline missing.");
    }

    #[test]
    fn trailing_blank_line() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n\n".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Trailing blank line detected.");
    }

    #[test]
    fn multiple_trailing_blank_lines() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n\n\n\n".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].message, "Trailing blank line detected.");
    }

    #[test]
    fn proper_final_newline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn multiline_proper() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\ny = 2\n".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn empty_file() {
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn final_blank_line_style_accepts_trailing_blank() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("final_blank_line".into()),
            )]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n\n".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &config, &mut diags, None);
        assert!(
            diags.is_empty(),
            "final_blank_line style should accept trailing blank line"
        );
    }

    #[test]
    fn final_blank_line_style_flags_missing_blank() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("final_blank_line".into()),
            )]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &config, &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "Trailing blank line missing.");
    }

    #[test]
    fn autocorrect_missing_newline() {
        let input = b"x = 1";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&TrailingEmptyLines, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }

    #[test]
    fn autocorrect_trailing_blank() {
        let input = b"x = 1\n\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&TrailingEmptyLines, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }

    #[test]
    fn autocorrect_multiple_trailing() {
        let input = b"x = 1\n\n\n\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&TrailingEmptyLines, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }

    #[test]
    fn autocorrect_final_blank_line_style_missing() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("final_blank_line".into()),
            )]),
            ..CopConfig::default()
        };
        let input = b"x = 1\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect_with_config(&TrailingEmptyLines, input, config);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n\n");
    }
}
