use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LineContinuationSpacing;

impl Cop for LineContinuationSpacing {
    fn name(&self) -> &'static str {
        "Layout/LineContinuationSpacing"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "space");

        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        for (i, &line) in lines.iter().enumerate() {
            // Strip trailing \r
            let trimmed_end = line
                .iter()
                .rposition(|&b| b != b'\r')
                .map(|p| &line[..=p])
                .unwrap_or(line);

            if !trimmed_end.ends_with(b"\\") {
                continue;
            }

            let backslash_pos = trimmed_end.len() - 1;

            match style {
                "space" => {
                    // Should have exactly one space before the backslash
                    if backslash_pos == 0 {
                        continue;
                    }
                    let before = trimmed_end[backslash_pos - 1];
                    if before != b' ' && before != b'\t' {
                        // No space before backslash
                        let line_num = i + 1;
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            backslash_pos,
                            "Use one space before backslash.".to_string(),
                        ));
                    } else if backslash_pos >= 2 && trimmed_end[backslash_pos - 2] == b' ' {
                        // Multiple spaces before backslash
                        let line_num = i + 1;
                        // Find start of spaces
                        let mut space_start = backslash_pos - 1;
                        while space_start > 0 && trimmed_end[space_start - 1] == b' ' {
                            space_start -= 1;
                        }
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            space_start,
                            "Use one space before backslash.".to_string(),
                        ));
                    }
                }
                "no_space" => {
                    // Should have no space before the backslash
                    if backslash_pos > 0
                        && (trimmed_end[backslash_pos - 1] == b' '
                            || trimmed_end[backslash_pos - 1] == b'\t')
                    {
                        let line_num = i + 1;
                        let mut space_start = backslash_pos - 1;
                        while space_start > 0
                            && (trimmed_end[space_start - 1] == b' '
                                || trimmed_end[space_start - 1] == b'\t')
                        {
                            space_start -= 1;
                        }
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            space_start,
                            "No space before backslash.".to_string(),
                        ));
                    }
                }
                _ => {}
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        LineContinuationSpacing,
        "cops/layout/line_continuation_spacing"
    );
}
