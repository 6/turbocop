use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct LineContinuationSpacing;

impl Cop for LineContinuationSpacing {
    fn name(&self) -> &'static str {
        "Layout/LineContinuationSpacing"
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
        let style = config.get_str("EnforcedStyle", "space");

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

            // Skip backslashes inside heredocs/strings
            let backslash_offset = line_starts[i] + backslash_pos;
            if !code_map.is_code(backslash_offset) {
                continue;
            }

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
