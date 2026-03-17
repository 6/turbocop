use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation
///
/// FN fix: was using `is_code()` to skip non-code regions, which excluded
/// `=begin`/`=end` multi-line comment blocks. RuboCop only skips string
/// literals (via `string_literal_ranges`), not comments. Changed to
/// `is_not_string()` to match RuboCop's behavior. This fixed 225 FN across
/// 8 corpus repos (WhatWeb: 136, greasyfork: 58, others: 31).
///
/// ## Corpus investigation (2026-03-17, FN=73)
///
/// 73 FN on heredoc closing delimiters with tab indentation (e.g., `\tSQL`).
/// Root cause: CodeMap maps heredoc ranges including the closing delimiter,
/// so `is_not_string()` returned false and the line was skipped. In Parser
/// gem, the closing delimiter is a separate `:tSTRING_END` token NOT
/// included in `string_literal_ranges`, so RuboCop checks its indentation.
/// Fix: detect heredoc closing delimiter lines (inside heredoc range,
/// content is just an identifier) and still check their indentation.
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

            // Skip lines whose indentation starts in a string/heredoc region.
            // RuboCop checks indentation in comments (including =begin/=end blocks)
            // but skips string literals, so use is_not_string() instead of is_code().
            // Exception: heredoc closing delimiters (e.g., `\tSQL`) are NOT skipped.
            // In Parser gem, the closing delimiter is a separate :tSTRING_END token
            // outside the string_literal_range, so RuboCop checks its indentation.
            if !code_map.is_not_string(line_start) {
                // Check if this line is a heredoc closing delimiter — if so, still check it.
                if !is_heredoc_closing_delimiter(line, code_map, line_start) {
                    continue;
                }
            }

            if style == "spaces" {
                // Flag tabs in indentation
                let indent_end = line
                    .iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .count();
                let indent = &line[..indent_end];
                if indent.contains(&b'\t') {
                    let tab_col = indent.iter().position(|&b| b == b'\t').unwrap_or(0);
                    let tab_offset = line_start + tab_col;
                    // Double-check the specific tab character is not in a string literal.
                    // Exception: heredoc closing delimiters are checked even though
                    // they're inside the heredoc range in the CodeMap.
                    if code_map.is_not_string(tab_offset)
                        || is_heredoc_closing_delimiter(line, code_map, line_start)
                    {
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
                if indent.contains(&b' ') {
                    let space_col = indent.iter().position(|&b| b == b' ').unwrap_or(0);
                    let space_offset = line_start + space_col;
                    if code_map.is_not_string(space_offset)
                        || is_heredoc_closing_delimiter(line, code_map, line_start)
                    {
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

/// Check if a line is a heredoc closing delimiter.
/// Heredoc closing delimiters are lines inside a heredoc range that contain only
/// a word (the delimiter) with optional leading whitespace (for `<<~` and `<<-`).
/// In Parser gem, the closing delimiter is a `:tSTRING_END` token and is NOT
/// included in `string_literal_ranges`, so RuboCop checks its indentation.
fn is_heredoc_closing_delimiter(line: &[u8], code_map: &CodeMap, line_start: usize) -> bool {
    // Must be inside a heredoc range (not just any string)
    if !code_map.is_heredoc(line_start) {
        return false;
    }

    // The line should look like a heredoc delimiter: optional whitespace + identifier only
    let trimmed = line.iter().skip_while(|&&b| b == b' ' || b == b'\t');
    let ident_len = trimmed
        .clone()
        .take_while(|&&b| b.is_ascii_alphanumeric() || b == b'_')
        .count();
    if ident_len == 0 {
        return false;
    }
    let content_len = line
        .iter()
        .skip_while(|&&b| b == b' ' || b == b'\t')
        .count();
    content_len == ident_len
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(IndentationStyle, "cops/layout/indentation_style");
    crate::cop_autocorrect_fixture_tests!(IndentationStyle, "cops/layout/indentation_style");

    #[test]
    fn heredoc_closing_tag_tab() {
        // Tab-indented heredoc closing tag should be flagged
        let source = b"execute <<-SQL\n\tSELECT * FROM users\n\tSQL\n";
        let diags = crate::testutil::run_cop_full(&IndentationStyle, source);
        assert!(
            !diags.is_empty(),
            "Should flag tab in heredoc closing tag indentation"
        );
        assert_eq!(
            diags.len(),
            1,
            "Only the closing tag tab, not heredoc content: {:?}",
            diags
        );
    }

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
