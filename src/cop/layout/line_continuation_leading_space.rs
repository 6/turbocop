use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct LineContinuationLeadingSpace;

impl Cop for LineContinuationLeadingSpace {
    fn name(&self) -> &'static str {
        "Layout/LineContinuationLeadingSpace"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "trailing");

        let content = source.as_bytes();
        let lines: Vec<&[u8]> = source.lines().collect();

        // Precompute byte offset of each line start
        let mut line_starts: Vec<usize> = Vec::with_capacity(lines.len());
        let mut offset = 0usize;
        for (i, line) in lines.iter().enumerate() {
            line_starts.push(offset);
            offset += line.len();
            if i < lines.len() - 1 || (offset < content.len() && content[offset] == b'\n') {
                offset += 1;
            }
        }

        for i in 0..lines.len().saturating_sub(1) {
            let line = lines[i];

            // Check if line ends with backslash continuation
            let trimmed_end = line
                .iter()
                .rposition(|&b| b != b'\r')
                .map(|p| &line[..=p])
                .unwrap_or(line);
            if !trimmed_end.ends_with(b"\\") {
                continue;
            }

            // Skip backslashes inside heredocs (but not regular string continuations,
            // which are the exact cases this cop inspects)
            let backslash_pos = trimmed_end.len() - 1;
            let backslash_offset = line_starts[i] + backslash_pos;
            if code_map.is_heredoc(backslash_offset) {
                continue;
            }

            // Check if this is a string continuation (line before \ ends with quote)
            let before_backslash = &trimmed_end[..trimmed_end.len() - 1];
            let before_trimmed = before_backslash
                .iter()
                .rposition(|&b| b != b' ' && b != b'\t')
                .map(|p| &before_backslash[..=p])
                .unwrap_or(before_backslash);

            // Only check string line continuations (end with ' or " before spaces+\)
            let is_string_continuation = before_trimmed.ends_with(b"'")
                || before_trimmed.ends_with(b"\"");
            if !is_string_continuation {
                continue;
            }

            let next_line = lines[i + 1];

            // Find the string opening on the next line
            let next_trimmed: Vec<u8> = next_line
                .iter()
                .copied()
                .skip_while(|&b| b == b' ' || b == b'\t')
                .collect();

            let is_next_string = next_trimmed.starts_with(b"'") || next_trimmed.starts_with(b"\"");
            if !is_next_string {
                continue;
            }

            match enforced_style {
                "trailing" => {
                    // In trailing style, leading spaces on next line are bad
                    // Check if next line string content starts with spaces
                    if next_trimmed.len() > 1 {
                        let after_quote = &next_trimmed[1..];
                        if after_quote.starts_with(b" ") {
                            let line_num = i + 2; // 1-based
                            let col = next_line
                                .iter()
                                .position(|&b| b != b' ' && b != b'\t')
                                .unwrap_or(0)
                                + 1; // After the quote
                            diagnostics.push(self.diagnostic(
                                source,
                                line_num,
                                col,
                                "Move leading spaces to the end of the previous line.".to_string(),
                            ));
                        }
                    }
                }
                "leading" => {
                    // In leading style, trailing spaces on current line are bad
                    if !before_trimmed.is_empty() {
                        let _quote_byte = before_trimmed[before_trimmed.len() - 1];
                        if before_trimmed.len() >= 2 {
                            let before_quote = &before_trimmed[..before_trimmed.len() - 1];
                            if before_quote.ends_with(b" ") {
                                let line_num = i + 1; // 1-based
                                // Find the position of trailing spaces before the closing quote
                                let spaces_start = before_quote
                                    .iter()
                                    .rposition(|&b| b != b' ')
                                    .map(|p| p + 1)
                                    .unwrap_or(0);
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line_num,
                                    spaces_start,
                                    "Move trailing spaces to the start of the next line."
                                        .to_string(),
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        LineContinuationLeadingSpace,
        "cops/layout/line_continuation_leading_space"
    );
}
