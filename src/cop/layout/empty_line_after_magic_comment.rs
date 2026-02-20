use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLineAfterMagicComment;

const MAGIC_COMMENT_PATTERNS: &[&str] = &[
    "frozen_string_literal:",
    "encoding:",
    "coding:",
    "warn_indent:",
    "shareable_constant_value:",
    "typed:",
];

fn is_magic_comment(line: &[u8]) -> bool {
    let trimmed = line.iter().position(|&b| b != b' ' && b != b'\t');
    let trimmed = match trimmed {
        Some(t) => &line[t..],
        None => return false,
    };
    if !trimmed.starts_with(b"#") {
        return false;
    }
    let after_hash = &trimmed[1..];
    let after_hash = if after_hash.starts_with(b" ") {
        &after_hash[1..]
    } else {
        after_hash
    };
    let line_str = std::str::from_utf8(after_hash).unwrap_or("");
    MAGIC_COMMENT_PATTERNS.iter().any(|p| line_str.starts_with(p))
}

impl Cop for EmptyLineAfterMagicComment {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineAfterMagicComment"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, _corrections: Option<&mut Vec<crate::correction::Correction>>) {
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut last_magic_line = None;

        for (i, line) in lines.iter().enumerate() {
            if is_magic_comment(line) {
                last_magic_line = Some(i);
            } else {
                // Stop at first non-magic-comment, non-blank line
                let is_blank = line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r');
                if !is_blank {
                    break;
                }
                // Blank lines between magic comments and code are fine
                if last_magic_line.is_some() {
                    break;
                }
            }
        }

        let last_magic_idx = match last_magic_line {
            Some(idx) => idx,
            None => return,
        };

        // Check if the line after the last magic comment is blank
        let next_idx = last_magic_idx + 1;
        if next_idx >= lines.len() {
            return;
        }

        let next_line = lines[next_idx];
        let is_blank = next_line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r');

        if !is_blank {
            diagnostics.push(self.diagnostic(
                source,
                next_idx + 1, // 1-indexed
                0,
                "Add an empty line after magic comments.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_scenario_fixture_tests!(
        EmptyLineAfterMagicComment,
        "cops/layout/empty_line_after_magic_comment",
        frozen_string = "frozen_string.rb",
        encoding = "encoding.rb",
        multiple_magic = "multiple_magic.rb",
    );
}
