use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct IndentationStyle;

impl Cop for IndentationStyle {
    fn name(&self) -> &'static str {
        "Layout/IndentationStyle"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "spaces");
        let indent_width = config.get_usize("IndentationWidth", 2);

        let mut offset = 0;

        for (i, line) in source.lines().enumerate() {
            let line_num = i + 1;
            let line_start = offset;
            // Advance offset past this line and its newline
            offset += line.len() + 1; // +1 for the '\n' delimiter

            // Skip lines whose indentation starts in a non-code region (heredocs, strings)
            if !code_map.is_code(line_start) {
                continue;
            }

            if style == "spaces" {
                // Flag tabs in indentation
                let indent_end = line
                    .iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .count();
                let indent = &line[..indent_end];
                if indent.iter().any(|&b| b == b'\t') {
                    let tab_col = indent.iter().position(|&b| b == b'\t').unwrap_or(0);
                    let tab_offset = line_start + tab_col;
                    // Double-check the specific tab character is in a code region
                    if code_map.is_code(tab_offset) {
                        let mut diag = self.diagnostic(
                            source,
                            line_num,
                            tab_col,
                            "Tab detected in indentation.".to_string(),
                        );
                        if let Some(ref mut corr) = corrections {
                            // Calculate visual width of the mixed indent region
                            let visual_width = indent.iter().fold(0usize, |w, &b| {
                                if b == b'\t' {
                                    (w / indent_width + 1) * indent_width
                                } else {
                                    w + 1
                                }
                            });
                            corr.push(crate::correction::Correction {
                                start: line_start,
                                end: line_start + indent_end,
                                replacement: " ".repeat(visual_width),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                        diagnostics.push(diag);
                    }
                }
            } else {
                // "tabs" — flag spaces in indentation
                let indent_end = line
                    .iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .count();
                let indent = &line[..indent_end];
                if indent.iter().any(|&b| b == b' ') {
                    let space_col = indent.iter().position(|&b| b == b' ').unwrap_or(0);
                    let space_offset = line_start + space_col;
                    if code_map.is_code(space_offset) {
                        let mut diag = self.diagnostic(
                            source,
                            line_num,
                            space_col,
                            "Space detected in indentation.".to_string(),
                        );
                        if let Some(ref mut corr) = corrections {
                            // Count leading spaces and convert to tabs
                            let space_count = indent.iter().filter(|&&b| b == b' ').count();
                            let tab_count = indent.iter().filter(|&&b| b == b'\t').count();
                            let total_tabs = tab_count + space_count / indent_width;
                            let remaining_spaces = space_count % indent_width;
                            let mut replacement = "\t".repeat(total_tabs);
                            replacement.push_str(&" ".repeat(remaining_spaces));
                            corr.push(crate::correction::Correction {
                                start: line_start,
                                end: line_start + indent_end,
                                replacement,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(IndentationStyle, "cops/layout/indentation_style");
    crate::cop_autocorrect_fixture_tests!(IndentationStyle, "cops/layout/indentation_style");

    #[test]
    fn autocorrect_tab_to_spaces() {
        let input = b"\tx = 1\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&IndentationStyle, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"  x = 1\n");
    }

    #[test]
    fn autocorrect_spaces_to_tab() {
        use std::collections::HashMap;
        let config = crate::cop::CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("tabs".into()),
            )]),
            ..crate::cop::CopConfig::default()
        };
        let input = b"  x = 1\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect_with_config(&IndentationStyle, input, config);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"\tx = 1\n");
    }
}
