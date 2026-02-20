use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct EmptyComment;

impl Cop for EmptyComment {
    fn name(&self) -> &'static str {
        "Layout/EmptyComment"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_border = config.get_bool("AllowBorderComment", true);
        let allow_margin = config.get_bool("AllowMarginComment", true);

        let lines: Vec<&[u8]> = source.lines().collect();

        if allow_margin {
            check_with_grouping(&lines, allow_border, source, code_map, self, diagnostics, corrections.as_deref_mut());
        } else {
            check_without_grouping(&lines, allow_border, source, code_map, self, diagnostics, corrections.as_deref_mut());
        }
    }
}

/// Classify a line: returns (indent_col, is_empty_comment, is_border_comment).
fn classify_line(line: &[u8]) -> Option<(usize, bool, bool)> {
    let col = line.iter().position(|&b| b != b' ' && b != b'\t')?;
    let content = &line[col..];
    if !content.starts_with(b"#") {
        return None;
    }
    let after_hash = &content[1..];
    let is_empty = !after_hash
        .iter()
        .any(|&b| b != b' ' && b != b'\t' && b != b'\r');
    // A border comment is "##", "###", etc. (2+ consecutive #'s, nothing else)
    let is_border = content.len() >= 2 && content.iter().all(|&b| b == b'#');
    Some((col, is_empty, is_border))
}

/// Check with consecutive-grouping (AllowMarginComment: true).
/// Groups consecutive comment lines at the same column, then only flags
/// groups where ALL comments are empty (or border-only if not allowed).
fn check_with_grouping(
    lines: &[&[u8]],
    allow_border: bool,
    source: &SourceFile,
    code_map: &CodeMap,
    cop: &EmptyComment,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
) {
    let total_len = source.as_bytes().len();

    // Build list of (line_index, column, is_empty, is_border, line_byte_offset, line_byte_len)
    // for comment lines. Skip lines inside non-code regions (heredocs, strings).
    let mut comment_lines: Vec<(usize, usize, bool, bool, usize, usize)> = Vec::new();
    let mut byte_offset: usize = 0;
    for (i, line) in lines.iter().enumerate() {
        let line_len = line.len() + 1; // +1 for newline
        if code_map.is_not_string(byte_offset) {
            if let Some((col, is_empty, is_border)) = classify_line(line) {
                comment_lines.push((i, col, is_empty, is_border, byte_offset, line_len));
            }
        }
        byte_offset += line_len;
    }

    // Group consecutive comments: same column, adjacent line numbers
    let mut group_start = 0;
    while group_start < comment_lines.len() {
        let mut group_end = group_start + 1;
        while group_end < comment_lines.len() {
            let prev = &comment_lines[group_end - 1];
            let curr = &comment_lines[group_end];
            // Same column and adjacent line
            if curr.1 == prev.1 && curr.0 == prev.0 + 1 {
                group_end += 1;
            } else {
                break;
            }
        }

        // Check if the entire group is only empty/border comments
        let group = &comment_lines[group_start..group_end];
        let all_empty_or_border = group.iter().all(|&(_, _, is_empty, is_border, _, _)| {
            is_empty || (!allow_border && is_border)
        });

        if all_empty_or_border {
            // Flag each empty/border comment in the group
            for &(line_idx, col, is_empty, is_border, line_offset, line_byte_len) in group {
                if is_empty || (!allow_border && is_border) {
                    let mut diag = cop.diagnostic(
                        source,
                        line_idx + 1,
                        col,
                        "Source code comment is empty.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        let end = std::cmp::min(line_offset + line_byte_len, total_len);
                        corr.push(crate::correction::Correction {
                            start: line_offset,
                            end,
                            replacement: String::new(),
                            cop_name: cop.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }
        }

        group_start = group_end;
    }
}

/// Check without grouping (AllowMarginComment: false).
fn check_without_grouping(
    lines: &[&[u8]],
    allow_border: bool,
    source: &SourceFile,
    code_map: &CodeMap,
    cop: &EmptyComment,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
) {
    let total_len = source.as_bytes().len();
    let mut byte_offset: usize = 0;

    for (i, line) in lines.iter().enumerate() {
        let line_len = line.len() + 1;
        if code_map.is_not_string(byte_offset) {
            if let Some((col, is_empty, is_border)) = classify_line(line) {
                if is_empty || (!allow_border && is_border) {
                    let mut diag = cop.diagnostic(
                        source,
                        i + 1,
                        col,
                        "Source code comment is empty.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        let end = std::cmp::min(byte_offset + line_len, total_len);
                        corr.push(crate::correction::Correction {
                            start: byte_offset,
                            end,
                            replacement: String::new(),
                            cop_name: cop.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }
        }
        byte_offset += line_len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(EmptyComment, "cops/layout/empty_comment");
    crate::cop_autocorrect_fixture_tests!(EmptyComment, "cops/layout/empty_comment");

    #[test]
    fn autocorrect_remove_empty_comment() {
        let input = b"x = 1\n#\ny = 2\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&EmptyComment, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\ny = 2\n");
    }

    #[test]
    fn skip_empty_comment_in_heredoc() {
        let source = b"x = <<~RUBY\n  #\n  code\nRUBY\n";
        let diags = crate::testutil::run_cop_full(&EmptyComment, source);
        assert!(diags.is_empty(), "Should not fire on # inside heredoc");
    }
}
