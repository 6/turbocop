use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyComment;

impl Cop for EmptyComment {
    fn name(&self) -> &'static str {
        "Layout/EmptyComment"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let allow_border = config.get_bool("AllowBorderComment", true);
        let allow_margin = config.get_bool("AllowMarginComment", true);

        let mut diagnostics = Vec::new();
        let lines: Vec<&[u8]> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            let trimmed = line.iter().position(|&b| b != b' ' && b != b'\t');
            let trimmed_start = match trimmed {
                Some(p) => p,
                None => continue,
            };
            let trimmed_content = &line[trimmed_start..];

            // Must start with #
            if !trimmed_content.starts_with(b"#") {
                continue;
            }

            // Check if the comment text (after #) is empty
            let after_hash = &trimmed_content[1..];
            let stripped = after_hash
                .iter()
                .position(|&b| b != b' ' && b != b'\t' && b != b'\r');

            if stripped.is_some() {
                // Not empty - but check if it's a border comment (all #'s)
                let is_all_hash = trimmed_content.iter().all(|&b| b == b'#');
                if !is_all_hash {
                    continue;
                }
                // It's a border comment like ####
                if allow_border {
                    continue;
                }
                // Not allowed - fall through to flag it
            } else {
                // Comment is just "#" possibly with trailing spaces
                // Check if it's a margin comment (# before/after a block of comments)
                if allow_margin && is_margin_comment(&lines, i) {
                    continue;
                }
            }

            diagnostics.push(self.diagnostic(
                source,
                line_num,
                trimmed_start,
                "Source code comment is empty.".to_string(),
            ));
        }

        diagnostics
    }
}

/// Check if a `#` line is a margin comment (adjacent to another comment line).
fn is_margin_comment(lines: &[&[u8]], line_idx: usize) -> bool {
    // Check if previous or next line is a non-empty comment
    if line_idx > 0 {
        if is_non_empty_comment(lines[line_idx - 1]) {
            return true;
        }
    }
    if line_idx + 1 < lines.len() {
        if is_non_empty_comment(lines[line_idx + 1]) {
            return true;
        }
    }
    false
}

/// Check if a line is a comment with actual content (not just `#`).
fn is_non_empty_comment(line: &[u8]) -> bool {
    let trimmed_start = match line.iter().position(|&b| b != b' ' && b != b'\t') {
        Some(p) => p,
        None => return false,
    };
    let content = &line[trimmed_start..];
    if !content.starts_with(b"#") {
        return false;
    }
    // Check if there's actual text after #
    let after_hash = &content[1..];
    let has_text = after_hash
        .iter()
        .any(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'#');
    has_text
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(EmptyComment, "cops/layout/empty_comment");
}
