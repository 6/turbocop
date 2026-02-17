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
        code_map: &CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _inspect_blocks = config.get_bool("InspectBlocks", false);
        let max_line_length = config.get_usize("MaxLineLength", 120);

        let content = source.as_bytes();
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        // Precompute byte offset of each line start.
        // source.lines() splits on '\n', so line i starts at cumulative offset.
        let mut line_starts: Vec<usize> = Vec::with_capacity(lines.len());
        let mut offset = 0usize;
        for (i, line) in lines.iter().enumerate() {
            line_starts.push(offset);
            offset += line.len();
            if i < lines.len() - 1 || (offset < content.len() && content[offset] == b'\n') {
                offset += 1;
            }
        }

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            let trimmed = trim_trailing_whitespace(line);

            if trimmed.is_empty() {
                i += 1;
                continue;
            }

            // Check if this line ends with a backslash continuation
            if !trimmed.ends_with(b"\\") || i + 1 >= lines.len() {
                i += 1;
                continue;
            }

            // Skip if the trimmed line is a comment (backslash in # ... is not continuation)
            let trimmed_content = trim_leading_whitespace(trimmed);
            if trimmed_content.starts_with(b"#") {
                i += 1;
                continue;
            }

            // Find the byte offset of the backslash to check if it's inside a
            // string/heredoc/regexp via the CodeMap.
            let backslash_offset = line_starts[i] + trimmed.len() - 1;
            if !code_map.is_code(backslash_offset) {
                i += 1;
                continue;
            }

            // Collect the full continuation group: all consecutive lines ending
            // with backslash.
            let group_start = i;
            let mut group_end = i;
            while group_end + 1 < lines.len() {
                let t = trim_trailing_whitespace(lines[group_end]);
                if !t.ends_with(b"\\") {
                    break;
                }
                let next_trimmed_content =
                    trim_leading_whitespace(trim_trailing_whitespace(lines[group_end + 1]));
                if next_trimmed_content.starts_with(b"#") {
                    break;
                }
                group_end += 1;
            }
            let final_line_idx = group_end + 1;
            if final_line_idx >= lines.len() {
                i = final_line_idx;
                continue;
            }

            // Build the combined single-line version.
            let indent = leading_whitespace_len(lines[group_start]);
            let mut combined = Vec::new();
            combined.extend_from_slice(&lines[group_start][..indent]);

            for j in group_start..=group_end {
                let t = trim_trailing_whitespace(lines[j]);
                let before_bs = trim_trailing_whitespace(&t[..t.len() - 1]);
                let content_part = trim_leading_whitespace(before_bs);

                if j == group_start {
                    combined.extend_from_slice(content_part);
                } else {
                    combined.push(b' ');
                    combined.extend_from_slice(content_part);
                }
            }

            let final_content = trim_leading_whitespace(lines[final_line_idx]);
            if !final_content.is_empty() {
                combined.push(b' ');
                combined.extend_from_slice(trim_trailing_whitespace(final_content));
            }

            let combined_len = combined.len();

            if combined_len > max_line_length {
                i = final_line_idx + 1;
                continue;
            }

            // Skip if next line starts with && or || (style choice)
            let next_content = trim_leading_whitespace(lines[group_start + 1]);
            if next_content.starts_with(b"&&") || next_content.starts_with(b"||") {
                i = final_line_idx + 1;
                continue;
            }

            diagnostics.push(self.diagnostic(
                source,
                group_start + 1,
                0,
                "Redundant line break detected.".to_string(),
            ));

            i = final_line_idx + 1;
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
