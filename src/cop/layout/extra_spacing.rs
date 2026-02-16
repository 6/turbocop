use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct ExtraSpacing;

impl Cop for ExtraSpacing {
    fn name(&self) -> &'static str {
        "Layout/ExtraSpacing"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_for_alignment = config.get_bool("AllowForAlignment", true);
        let allow_before_trailing_comments = config.get_bool("AllowBeforeTrailingComments", false);
        let _force_equal_sign_alignment = config.get_bool("ForceEqualSignAlignment", false);

        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        // Track cumulative byte offset for each line start
        let mut line_start_offset: usize = 0;

        for (line_idx, &line) in lines.iter().enumerate() {
            let line_num = line_idx + 1;
            let mut i = 0;

            // Skip leading whitespace (indentation)
            while i < line.len() && (line[i] == b' ' || line[i] == b'\t') {
                i += 1;
            }

            // Now scan for extra spaces within the line
            while i < line.len() {
                if line[i] == b' ' {
                    let space_start = i;
                    while i < line.len() && line[i] == b' ' {
                        i += 1;
                    }
                    let space_count = i - space_start;

                    if space_count > 1 && i < line.len() {
                        // Get the byte offset in the full source
                        let abs_offset = line_start_offset + space_start;

                        // Skip if inside string/comment
                        if !code_map.is_code(abs_offset) {
                            continue;
                        }

                        // Skip if before trailing comment and that's allowed
                        if allow_before_trailing_comments && line[i] == b'#' {
                            continue;
                        }

                        // Skip if this could be alignment
                        // Check if the token AFTER the spaces aligns with adjacent lines
                        if allow_for_alignment
                            && is_aligned_with_adjacent(&lines, line_idx, i)
                        {
                            continue;
                        }

                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            space_start + 1, // point to the first extra space
                            "Unnecessary spacing detected.".to_string(),
                        ));
                    }
                } else {
                    i += 1;
                }
            }

            // Advance to next line: line content + 1 for '\n'
            line_start_offset += line.len() + 1;
        }

        diagnostics
    }
}

/// Check if the token at the given position aligns with a token on an adjacent line.
fn is_aligned_with_adjacent(lines: &[&[u8]], line_idx: usize, col: usize) -> bool {
    // Check the line above and below for a non-space character at the same column
    let check_indices: [Option<usize>; 2] = [
        if line_idx > 0 {
            Some(line_idx - 1)
        } else {
            None
        },
        if line_idx + 1 < lines.len() {
            Some(line_idx + 1)
        } else {
            None
        },
    ];

    for adj_idx in check_indices.into_iter().flatten() {
        let adj_line = lines[adj_idx];
        // Skip blank lines
        if adj_line
            .iter()
            .all(|&b| b == b' ' || b == b'\t' || b == b'\r')
        {
            continue;
        }
        if col < adj_line.len() && adj_line[col] != b' ' && adj_line[col] != b'\t' {
            // Check that the character before is a space (so this is indeed aligning)
            if col > 0 && (adj_line[col - 1] == b' ' || adj_line[col - 1] == b'\t') {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ExtraSpacing, "cops/layout/extra_spacing");
}
