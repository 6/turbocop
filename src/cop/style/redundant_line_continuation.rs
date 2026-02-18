use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantLineContinuation;

impl Cop for RedundantLineContinuation {
    fn name(&self) -> &'static str {
        "Style/RedundantLineContinuation"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
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

            // Skip if inside a string (very basic heuristic: check if line starts inside a string)
            // We use a simple heuristic: line continuation after certain operators is redundant
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

        diagnostics
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
