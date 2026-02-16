use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CommentIndentation;

impl Cop for CommentIndentation {
    fn name(&self) -> &'static str {
        "Layout/CommentIndentation"
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let _allow_for_alignment = config.get_bool("AllowForAlignment", false);
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.iter().position(|&b| b != b' ' && b != b'\t');
            let trimmed = match trimmed {
                Some(t) => t,
                None => continue, // blank line
            };

            // Only check comment-only lines
            if line[trimmed] != b'#' {
                continue;
            }

            // Check if inline comment (has code before #) -- skip those
            // This is a standalone comment since we checked first non-ws is #

            let comment_col = trimmed;

            // Find the next non-blank, non-comment line to check alignment
            let mut next_code_col = None;
            for j in (i + 1)..lines.len() {
                let next_trimmed = lines[j].iter().position(|&b| b != b' ' && b != b'\t');
                if let Some(nt) = next_trimmed {
                    // Skip blank lines
                    if lines[j][nt] == b'#' {
                        // Another comment, skip
                        continue;
                    }
                    next_code_col = Some(nt);
                    break;
                }
            }

            if let Some(expected) = next_code_col {
                if comment_col != expected {
                    diagnostics.push(self.diagnostic(
                        source,
                        i + 1,
                        comment_col,
                        format!(
                            "Incorrect indentation detected (column {} instead of column {}).",
                            comment_col, expected
                        ),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(CommentIndentation, "cops/layout/comment_indentation");
}
