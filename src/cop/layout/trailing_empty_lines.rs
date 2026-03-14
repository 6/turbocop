use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=47, FN=22.
///
/// Fixed sampled FN: whitespace-only lines at EOF were previously excluded
/// from the trailing-blank-line scan, so the default `final_newline` style
/// could miss `Trailing blank line detected.` and sometimes fall through to
/// `Final newline missing.` instead. The accepted fix treats space/tab/CR-only
/// lines as blank and autocorrect removes the full blank tail.
///
/// Acceptance gate after this patch (`scripts/check-cop.py --verbose --rerun`):
/// expected=9,691, actual=9,587, CI baseline=9,716, excess=0, missing=104,
/// file-drop noise=308.
///
/// Remaining gap: 104 potential FN remain. This batch only addressed the
/// whitespace-only trailing-line case; no broader config-resolution issue was
/// involved.
///
/// ## Corpus investigation (2026-03-14)
///
/// FP=62, FN=6. Two missing exemptions from RuboCop:
/// 1. Files containing `__END__` anywhere are skipped by RuboCop (regex
///    `/\s*__END__/`). Nitrocop was flagging trailing blank lines after
///    `__END__`, causing false positives.
/// 2. Files ending with `"%\n\n"` (percent blank string edge case) are
///    skipped by RuboCop. Nitrocop was flagging these as trailing blanks.
///
/// Both exemptions now implemented.
pub struct TrailingEmptyLines;

/// Check if the source contains `__END__` (with optional leading whitespace).
/// Matches RuboCop's `/\s*__END__/` regex which matches anywhere in the file.
fn contains_end_marker(bytes: &[u8]) -> bool {
    // Scan for __END__ preceded only by whitespace on each line
    // The RuboCop regex /\s*__END__/ matches __END__ anywhere in the source
    // with optional leading whitespace characters
    bytes.windows(7).any(|w| w == b"__END__")
}

fn is_blank_line(line: &[u8]) -> bool {
    line.iter().all(|&b| matches!(b, b' ' | b'\t' | b'\r'))
}

fn trailing_blank_lines(lines: &[&[u8]], ends_with_newline: bool) -> Option<(usize, usize)> {
    let mut first_non_blank_after_tail = lines.len();
    while first_non_blank_after_tail > 0 && is_blank_line(lines[first_non_blank_after_tail - 1]) {
        first_non_blank_after_tail -= 1;
    }

    let trailing_slices = lines.len().saturating_sub(first_non_blank_after_tail);
    let trailing_blank_lines = if ends_with_newline {
        trailing_slices.saturating_sub(1)
    } else {
        trailing_slices
    };

    (trailing_blank_lines > 0).then_some((trailing_blank_lines, first_non_blank_after_tail + 1))
}

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

        // RuboCop skips files containing __END__ (with optional leading whitespace)
        if contains_end_marker(bytes) {
            return;
        }

        // RuboCop skips files ending with "%\n\n" (percent blank string edge case)
        if bytes.ends_with(b"%\n\n") {
            return;
        }
        let lines: Vec<&[u8]> = source.lines().collect();
        let ends_with_newline = bytes.ends_with(b"\n");

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
                if let Some((_count, first_blank_line)) =
                    trailing_blank_lines(&lines, ends_with_newline)
                {
                    let mut diag = self.diagnostic(
                        source,
                        first_blank_line,
                        0,
                        "Trailing blank line detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        if let Some(start) = source.line_col_to_offset(first_blank_line, 0) {
                            corr.push(crate::correction::Correction {
                                start,
                                end: bytes.len(),
                                replacement: String::new(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                    }
                    diagnostics.push(diag);
                    return;
                }

                if !ends_with_newline {
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
        whitespace_trailing = "whitespace_trailing.rb",
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
    fn skip_file_with_end_marker() {
        // RuboCop skips files containing __END__ anywhere
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n__END__\nsome data\n\n".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty(), "should skip files containing __END__");
    }

    #[test]
    fn skip_file_with_end_marker_leading_whitespace() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n  __END__\nsome data\n\n".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "should skip files with __END__ preceded by whitespace"
        );
    }

    #[test]
    fn skip_file_ending_with_percent_blank_string() {
        // RuboCop skips files ending with "%\n\n"
        let source = SourceFile::from_bytes("test.rb", b"x = \"%\n\n".to_vec());
        let mut diags = Vec::new();
        TrailingEmptyLines.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "should skip files ending with percent blank string"
        );
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
