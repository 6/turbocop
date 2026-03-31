use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/CommentIndentation: checks that comments are indented correctly.
///
/// ## Investigation findings (2026-03-08)
///
/// Root cause of 6,355 FPs: nitrocop was skipping comment lines when looking for
/// the "next line" to determine expected indentation, while RuboCop uses the next
/// non-blank line regardless of whether it's a comment or code. This caused massive
/// FPs when comment blocks appeared before code at a different indentation level —
/// every comment in the block was checked against the distant code line instead of
/// the immediately following comment line.
///
/// Fix: Changed to match RuboCop's `line_after_comment` algorithm — find the next
/// non-blank line (including other comments). Also added handling for:
/// - Comments at end of file with no following line (expected indent = 0)
/// - The `is_less_indented` and `is_two_alternative_keyword` checks only apply
///   when the next non-blank line is actual code (not another comment)
///
/// ## Follow-up fix: 26 more FPs (2026-03-08)
///
/// Two additional bugs:
/// 1. `\r` from Windows `\r\n` line endings not treated as whitespace in blank-line
///    detection. Lines split by `\n` retain trailing `\r`, which was treated as
///    non-blank content at column 0, corrupting expected indentation. (17 FPs)
/// 2. `elsif(` (parenthesized condition without space) not recognized as a
///    two-alternative keyword. Only `elsif ` and `elsif\n` were checked. (9 FPs)
///
/// ## Follow-up fix: 3 FPs from `else;` pattern (2026-03-08)
///
/// `else; fail 'not raised'` (semicolon-separated statement on same line as `else`)
/// was not recognized by `is_two_alternative_keyword`. The function checked for
/// `else\n`, `else\r`, `else `, and bare `else`, but not `else;`. Fixed by using
/// a general delimiter check: `else` followed by any non-alphanumeric, non-underscore
/// character (matching the pattern already used for `end` in `is_less_indented`).
///
/// ## Follow-up fix: own-line comments after leading interpolation (2026-03-31)
///
/// Real FN: `%Q;\\` bodies can contain lines like ` #{2**2}; #=> " 4"`. RuboCop
/// still checks the trailing `#=>` here because `own_line_comment?` only asks
/// whether the physical line starts with `#` after indentation; it does not
/// require that leading `#` to be the actual comment token. Our implementation
/// incorrectly coupled those two checks, so it skipped any line where the first
/// visible `#` belonged to interpolation and the real comment started later.
///
/// Fix: track Prism's actual comment columns per line, use RuboCop's line-prefix
/// rule to decide whether the line counts as an "own-line comment", and then
/// report against the real comment column from Prism.
pub struct CommentIndentation;

fn first_non_whitespace_column(line: &[u8]) -> Option<usize> {
    line.iter()
        .position(|&b| b != b' ' && b != b'\t' && b != b'\r')
}

/// Check if a line starts with one of the "two alternative" keywords.
/// When a comment precedes one of these, it can be indented to match either
/// the keyword or the body it precedes (keyword indent + indentation_width).
fn is_two_alternative_keyword(line: &[u8]) -> bool {
    let trimmed: &[u8] = &line[line
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(line.len())..];
    trimmed.starts_with(b"else")
        && (trimmed.len() == 4 || !trimmed[4].is_ascii_alphanumeric() && trimmed[4] != b'_')
        || trimmed.starts_with(b"elsif ")
        || trimmed.starts_with(b"elsif\n")
        || trimmed.starts_with(b"elsif(")
        || trimmed.starts_with(b"when ")
        || trimmed.starts_with(b"when\n")
        || trimmed.starts_with(b"in ")
        || trimmed.starts_with(b"in\n")
        || trimmed.starts_with(b"rescue")
        || trimmed.starts_with(b"ensure")
}

/// Check if a line is "less indented" — `end`, `)`, `}`, `]`.
/// Comments before these should align with the body, not the closing keyword.
fn is_less_indented(line: &[u8]) -> bool {
    let trimmed: &[u8] = &line[line
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(line.len())..];
    trimmed.starts_with(b"end")
        && (trimmed.len() == 3 || !trimmed[3].is_ascii_alphanumeric() && trimmed[3] != b'_')
        || trimmed.starts_with(b")")
        || trimmed.starts_with(b"}")
        || trimmed.starts_with(b"]")
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_for_alignment = config.get_bool("AllowForAlignment", false);
        let indent_width = config.get_usize("IndentationWidth", 2);
        let lines: Vec<&[u8]> = source.lines().collect();

        // Track actual comment columns per line from Prism so we can distinguish
        // real comments from `#` inside strings/regex/heredocs while still
        // matching RuboCop's line-based own-line comment check.
        let mut comment_columns_by_line: Vec<Vec<usize>> = vec![Vec::new(); lines.len()];
        for comment in parse_result.comments() {
            let (line, column) = source.offset_to_line_col(comment.location().start_offset());
            let line_idx = line - 1;
            if let Some(columns) = comment_columns_by_line.get_mut(line_idx) {
                columns.push(column);
            }
        }

        for (i, line) in lines.iter().enumerate() {
            let trimmed = first_non_whitespace_column(line);
            let trimmed = match trimmed {
                Some(t) => t,
                None => continue, // blank line
            };

            // Match RuboCop's `own_line_comment?`: the line must start with `#`
            // after indentation, and Prism must confirm there is a real comment
            // somewhere on the line.
            if line[trimmed] != b'#' || comment_columns_by_line[i].is_empty() {
                continue;
            }

            for &comment_col in &comment_columns_by_line[i] {
                // Find the next non-blank line (including comments).
                // This matches RuboCop's `line_after_comment` which finds the first
                // non-blank line, regardless of whether it's a comment or code.
                let mut next_line: Option<&[u8]> = None;
                let mut next_col = None;
                let mut next_line_idx = 0;
                for (j, ln) in lines.iter().enumerate().skip(i + 1) {
                    let next_trimmed = first_non_whitespace_column(ln);
                    if let Some(nt) = next_trimmed {
                        next_line = Some(ln);
                        next_col = Some(nt);
                        next_line_idx = j;
                        break;
                    }
                }

                // When no next line exists, expected indentation is 0
                // (matches RuboCop: `return 0 unless next_line`)
                let (expected, next_is_code) = if let Some(nc) = next_col {
                    let nl = next_line.unwrap();
                    // Apply the same own-line-comment rule to the next line.
                    let is_comment =
                        nl[nc] == b'#' && !comment_columns_by_line[next_line_idx].is_empty();
                    let exp = if !is_comment && is_less_indented(nl) {
                        nc + indent_width
                    } else {
                        nc
                    };
                    (exp, !is_comment)
                } else {
                    (0, false)
                };

                if comment_col == expected {
                    continue;
                }

                // Two-alternative keywords: comment can match keyword indent OR body indent
                // Only applies when next line is code (not a comment)
                if next_is_code {
                    if let Some(nl) = next_line {
                        if is_two_alternative_keyword(nl) {
                            let nc = next_col.unwrap();
                            let alt = nc + indent_width;
                            if comment_col == nc || comment_col == alt {
                                continue;
                            }
                        }
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
                        let prev_first = first_non_whitespace_column(prev);
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

    #[test]
    fn crlf_blank_lines_not_treated_as_content() {
        // \r\n line endings: after splitting on \n, blank lines are just \r
        // which must be treated as blank (not content at column 0)
        let source = b"def foo\r\n  # comment\r\n\r\n  x = 1\r\nend\r\n";
        crate::testutil::assert_cop_no_offenses(&CommentIndentation, source);
    }
}
