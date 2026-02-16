use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct RedundantLineBreak;

impl Cop for RedundantLineBreak {
    fn name(&self) -> &'static str {
        "Layout/RedundantLineBreak"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // RedundantLineBreak is disabled by default and has complex single-line
        // suitability logic (checking Layout/LineLength max, string concatenation,
        // block inspection, etc.). Implementing the full detection requires
        // integrating with LineLength config and doing expression reconstruction.
        //
        // For now, detect the simplest and most common pattern:
        // A line ending with a backslash where the current + next line (minus the
        // backslash and leading whitespace) would fit within a reasonable length.
        //
        // This is intentionally conservative to avoid false positives.
        let _inspect_blocks = _config.get_bool("InspectBlocks", false);

        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        let mut skip_next = false;
        for (i, line) in lines.iter().enumerate() {
            if skip_next {
                skip_next = false;
                continue;
            }

            let trimmed = trim_trailing_whitespace(line);
            if trimmed.is_empty() {
                continue;
            }

            // Detect backslash continuation lines
            if trimmed.ends_with(b"\\") && i + 1 < lines.len() {
                let before_backslash = &trimmed[..trimmed.len() - 1];
                let before_backslash = trim_trailing_whitespace(before_backslash);
                let next_line = trim_leading_whitespace(lines[i + 1]);

                // Check if the combined line would be reasonably short
                // We use the indentation of the current line + content
                let indent = leading_whitespace_len(line);
                let combined_len = indent + before_backslash.len() - indent + 1 + next_line.len();
                // Use the actual line content length (indentation + before_backslash content + space + next_line)
                let actual_combined = before_backslash.len() + 1 + next_line.len();
                let _ = combined_len;

                // Skip if combined would be too long (> 120 chars as a safe default)
                if actual_combined <= 120 {
                    // Additional check: don't flag operator keywords (&&, ||) after backslash
                    // as those are separate style choices
                    if !next_line.starts_with(b"&&") && !next_line.starts_with(b"||") {
                        diagnostics.push(self.diagnostic(
                            source,
                            i + 1, // 1-based
                            0,
                            "Redundant line break detected.".to_string(),
                        ));
                        skip_next = true;
                    }
                }
            }
        }

        diagnostics
    }
}

fn trim_trailing_whitespace(line: &[u8]) -> &[u8] {
    let mut end = line.len();
    while end > 0 && (line[end - 1] == b' ' || line[end - 1] == b'\t') {
        end -= 1;
    }
    &line[..end]
}

fn trim_leading_whitespace(line: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < line.len() && (line[start] == b' ' || line[start] == b'\t') {
        start += 1;
    }
    &line[start..]
}

fn leading_whitespace_len(line: &[u8]) -> usize {
    let mut count = 0;
    for &b in line {
        if b == b' ' || b == b'\t' {
            count += 1;
        } else {
            break;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        RedundantLineBreak,
        "cops/layout/redundant_line_break"
    );
}
