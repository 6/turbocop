use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantLineContinuation;

impl Cop for RedundantLineContinuation {
    fn name(&self) -> &'static str {
        "Style/RedundantLineContinuation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let lines: Vec<&[u8]> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = trim_end(line);
            if !trimmed.ends_with(b"\\") {
                continue;
            }

            // Check the character before backslash is not another backslash (string escape)
            if trimmed.len() >= 2 && trimmed[trimmed.len() - 2] == b'\\' {
                continue;
            }

            // Compute the absolute offset of the backslash to check if it's in code
            // We need to find the byte offset of this line's start + column
            let line_start = {
                let src = source.as_bytes();
                let mut offset = 0;
                let mut line_num = 0;
                for &b in src.iter() {
                    if line_num == i {
                        break;
                    }
                    offset += 1;
                    if b == b'\n' {
                        line_num += 1;
                    }
                }
                offset
            };
            let backslash_offset = line_start + trimmed.len() - 1;

            // Use code_map to verify the backslash is in a code region
            // (not inside a string, heredoc, or comment)
            if !code_map.is_code(backslash_offset) {
                continue;
            }

            let before_backslash = trim_end(&trimmed[..trimmed.len() - 1]);

            // Check if the continuation is after an operator or opening bracket
            // where Ruby would naturally continue to the next line
            if is_redundant_continuation(before_backslash, i, &lines) {
                let col = trimmed.len() - 1;
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    col,
                    "Redundant line continuation.".to_string(),
                ));
            }
        }

    }
}

fn trim_end(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
        end -= 1;
    }
    &bytes[..end]
}

fn is_redundant_continuation(before_backslash: &[u8], _line_idx: usize, _lines: &[&[u8]]) -> bool {
    let trimmed = trim_end(before_backslash);
    if trimmed.is_empty() {
        return false;
    }

    let last_byte = trimmed[trimmed.len() - 1];

    // After operators and opening brackets, continuation is redundant
    matches!(
        last_byte,
        b',' | b'(' | b'[' | b'{' | b'+' | b'-' | b'*' | b'/' | b'|' | b'&' | b'.'
        | b'=' | b'>' | b'<' | b'\\' | b':'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantLineContinuation, "cops/style/redundant_line_continuation");
}
