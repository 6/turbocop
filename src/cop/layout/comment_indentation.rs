use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CommentIndentation;

/// Check if a line starts with one of the "two alternative" keywords.
/// When a comment precedes one of these, it can be indented to match either
/// the keyword or the body it precedes (keyword indent + indentation_width).
fn is_two_alternative_keyword(line: &[u8]) -> bool {
    let trimmed: &[u8] = &line[line.iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(line.len())..];
    trimmed.starts_with(b"else\n") || trimmed.starts_with(b"else\r") || trimmed == b"else"
        || trimmed.starts_with(b"else ")
        || trimmed.starts_with(b"elsif ")  || trimmed.starts_with(b"elsif\n")
        || trimmed.starts_with(b"when ")   || trimmed.starts_with(b"when\n")
        || trimmed.starts_with(b"in ")     || trimmed.starts_with(b"in\n")
        || trimmed.starts_with(b"rescue")
        || trimmed.starts_with(b"ensure")
}

/// Check if a line is "less indented" — `end`, `)`, `}`, `]`.
/// Comments before these should align with the body, not the closing keyword.
fn is_less_indented(line: &[u8]) -> bool {
    let trimmed: &[u8] = &line[line.iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(line.len())..];
    trimmed.starts_with(b"end") && (trimmed.len() == 3 || !trimmed[3].is_ascii_alphanumeric() && trimmed[3] != b'_')
        || trimmed.starts_with(b")") || trimmed.starts_with(b"}") || trimmed.starts_with(b"]")
}

impl Cop for CommentIndentation {
    fn name(&self) -> &'static str {
        "Layout/CommentIndentation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let allow_for_alignment = config.get_bool("AllowForAlignment", false);
        let indent_width = config.get_usize("IndentationWidth", 2);
        let lines: Vec<&[u8]> = source.lines().collect();

        // Build set of byte offsets where real Ruby comments start.
        // This lets us distinguish actual comments from `#` inside strings/regex/heredocs.
        let mut comment_starts: HashSet<usize> = HashSet::new();
        for comment in parse_result.comments() {
            comment_starts.insert(comment.location().start_offset());
        }

        // Compute line start byte offsets
        let bytes = source.as_bytes();
        let mut line_offsets: Vec<usize> = vec![0];
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                line_offsets.push(i + 1);
            }
        }

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.iter().position(|&b| b != b' ' && b != b'\t');
            let trimmed = match trimmed {
                Some(t) => t,
                None => continue, // blank line
            };

            // Only check lines starting with #
            if line[trimmed] != b'#' {
                continue;
            }

            // Verify this # is an actual Ruby comment (not inside string/regex/heredoc)
            let hash_offset = line_offsets[i] + trimmed;
            if !comment_starts.contains(&hash_offset) {
                continue;
            }

            let comment_col = trimmed;

            // Find the next non-blank, non-comment line to check alignment
            let mut next_code_line: Option<&[u8]> = None;
            let mut next_code_col = None;
            for j in (i + 1)..lines.len() {
                let next_trimmed = lines[j].iter().position(|&b| b != b' ' && b != b'\t');
                if let Some(nt) = next_trimmed {
                    if lines[j][nt] == b'#' {
                        // Check if this # is a real comment or inside string/regex
                        let next_hash_offset = line_offsets[j] + nt;
                        if comment_starts.contains(&next_hash_offset) {
                            continue; // skip other comments
                        }
                    }
                    next_code_line = Some(lines[j]);
                    next_code_col = Some(nt);
                    break;
                }
            }

            if let Some(next_col) = next_code_col {
                let next_line = next_code_line.unwrap();

                // Calculate the correct expected indentation.
                // If the next line is `end`, `)`, `}`, `]`, the comment should
                // align with the body (one indent level deeper).
                let expected = if is_less_indented(next_line) {
                    next_col + indent_width
                } else {
                    next_col
                };

                if comment_col == expected {
                    continue;
                }

                // Two-alternative keywords: comment can match keyword indent OR body indent
                if is_two_alternative_keyword(next_line) {
                    let alt = next_col + indent_width;
                    if comment_col == next_col || comment_col == alt {
                        continue;
                    }
                }

                // AllowForAlignment: if enabled, check if this comment is aligned
                // with a preceding inline (end-of-line) comment
                if allow_for_alignment {
                    let mut aligned_with_preceding = false;
                    // Walk backwards through preceding comments looking for an
                    // end-of-line comment at the same column
                    for k in (0..i).rev() {
                        let prev = lines[k];
                        let prev_first = prev.iter().position(|&b| b != b' ' && b != b'\t');
                        match prev_first {
                            Some(pos) if prev[pos] == b'#' => {
                                // own-line comment — skip
                                continue;
                            }
                            Some(_) => {
                                // code line — check if it has an inline comment at our column
                                if let Some(hash_pos) = prev.iter().position(|&b| b == b'#') {
                                    if hash_pos == comment_col {
                                        aligned_with_preceding = true;
                                    }
                                }
                                break;
                            }
                            None => break, // blank line
                        }
                    }
                    if aligned_with_preceding {
                        continue;
                    }
                }

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
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(CommentIndentation, "cops/layout/comment_indentation");
}
