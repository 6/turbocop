use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// Earlier fixes removed the original FP=6/FN=56 gap by skipping block-comment
/// regions and grouping aligned inline comments instead of only standalone `#`
/// rows.
///
/// The remaining FN=3 on the current corpus baseline were inline empty comments
/// after interpolated strings, e.g. `"#{value}" #`. Raw byte scanning stopped at
/// the interpolation opener `#{` because Prism leaves embedded-statement syntax
/// outside `string_ranges`, so the real trailing comment was never reached.
///
/// The current implementation classifies comments from `parse_result.comments()`
/// and only uses raw line inspection to determine whether the parsed comment is
/// empty, a border comment, or a standalone/margin comment. This preserves the
/// earlier block-comment and aligned-margin fixes while matching inline comments
/// after interpolation.
///
/// Acceptance gate after the fix: expected 571, actual 609, CI baseline 568,
/// raw delta +41, file-drop noise 79, missing 0. The rerun passed because the
/// delta stayed within existing RuboCop parser-crash noise.
pub struct EmptyComment;

#[derive(Clone, Copy)]
struct CommentLine {
    line_idx: usize,
    col: usize,
    is_empty: bool,
    is_border: bool,
    is_standalone: bool,
    line_offset: usize,
    line_len: usize,
    line_span_len: usize,
    comment_offset: usize,
}

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
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_border = config.get_bool("AllowBorderComment", true);
        let allow_margin = config.get_bool("AllowMarginComment", true);

        let lines: Vec<&[u8]> = source.lines().collect();

        if allow_margin {
            check_with_grouping(
                &lines,
                allow_border,
                source,
                parse_result,
                self,
                diagnostics,
                corrections.as_deref_mut(),
            );
        } else {
            check_without_grouping(
                &lines,
                allow_border,
                source,
                parse_result,
                self,
                diagnostics,
                corrections,
            );
        }
    }
}

fn classify_comment(line: &[u8], col: usize) -> Option<(bool, bool, bool)> {
    if col >= line.len() || line[col] != b'#' {
        return None;
    }
    let is_standalone = line[..col].iter().all(|&b| b == b' ' || b == b'\t');
    let content = &line[col..];
    let after_hash = content.get(1..).unwrap_or_default();
    let is_empty = !after_hash
        .iter()
        .any(|&b| b != b' ' && b != b'\t' && b != b'\r');
    let is_border = is_standalone && content.len() >= 2 && content.iter().all(|&b| b == b'#');
    Some((is_empty, is_border, is_standalone))
}

fn is_block_comment_marker(line: &[u8], marker: &[u8]) -> bool {
    let start = line
        .iter()
        .position(|&b| b != b' ' && b != b'\t')
        .unwrap_or(line.len());
    line[start..].starts_with(marker)
}

fn collect_comment_lines(
    source: &SourceFile,
    lines: &[&[u8]],
    parse_result: &ruby_prism::ParseResult<'_>,
) -> Vec<CommentLine> {
    let bytes = source.as_bytes();
    let total_len = bytes.len();
    let mut comments = Vec::new();
    let mut byte_offset = 0usize;
    let mut in_block_comment = false;
    let mut comment_offsets = vec![None; lines.len()];

    for comment in parse_result.comments() {
        let comment_offset = comment.location().start_offset();
        let (line_num, _) = source.offset_to_line_col(comment_offset);
        if (1..=lines.len()).contains(&line_num) {
            comment_offsets[line_num - 1] = Some(comment_offset);
        }
    }

    for (i, line) in lines.iter().enumerate() {
        let line_len = line.len();
        let has_newline =
            byte_offset + line_len < total_len && bytes[byte_offset + line_len] == b'\n';
        let line_span_len = line_len + usize::from(has_newline);

        if in_block_comment {
            if is_block_comment_marker(line, b"=end") {
                in_block_comment = false;
            }
            byte_offset += line_span_len;
            continue;
        }

        if is_block_comment_marker(line, b"=begin") {
            in_block_comment = true;
            byte_offset += line_span_len;
            continue;
        }

        if let Some(comment_offset) = comment_offsets[i] {
            let col = comment_offset.saturating_sub(byte_offset);
            let Some((is_empty, is_border, is_standalone)) = classify_comment(line, col) else {
                byte_offset += line_span_len;
                continue;
            };
            comments.push(CommentLine {
                line_idx: i,
                col,
                is_empty,
                is_border,
                is_standalone,
                line_offset: byte_offset,
                line_len,
                line_span_len,
                comment_offset,
            });
        }

        byte_offset += line_span_len;
    }

    comments
}

fn push_comment_diagnostic(
    source: &SourceFile,
    cop: &EmptyComment,
    comment: CommentLine,
    total_len: usize,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
) {
    let mut diag = cop.diagnostic(
        source,
        comment.line_idx + 1,
        comment.col,
        "Source code comment is empty.".to_string(),
    );

    if let Some(ref mut corr) = corrections {
        let bytes = source.as_bytes();
        let (start, end) = if comment.is_standalone {
            (
                comment.line_offset,
                std::cmp::min(comment.line_offset + comment.line_span_len, total_len),
            )
        } else {
            let mut start = comment.comment_offset;
            while start > comment.line_offset && matches!(bytes[start - 1], b' ' | b'\t') {
                start -= 1;
            }
            (
                start,
                std::cmp::min(comment.line_offset + comment.line_len, total_len),
            )
        };

        corr.push(crate::correction::Correction {
            start,
            end,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }

    diagnostics.push(diag);
}

/// Check with consecutive-grouping (AllowMarginComment: true).
/// Groups consecutive comments at the same column, then only flags groups where
/// every aligned comment is empty (or border-only if borders are disallowed).
fn check_with_grouping(
    lines: &[&[u8]],
    allow_border: bool,
    source: &SourceFile,
    parse_result: &ruby_prism::ParseResult<'_>,
    cop: &EmptyComment,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
) {
    let total_len = source.as_bytes().len();
    let comment_lines = collect_comment_lines(source, lines, parse_result);

    let mut group_start = 0;
    while group_start < comment_lines.len() {
        let mut group_end = group_start + 1;
        while group_end < comment_lines.len() {
            let prev = comment_lines[group_end - 1];
            let curr = comment_lines[group_end];
            if curr.col == prev.col && curr.line_idx == prev.line_idx + 1 {
                group_end += 1;
            } else {
                break;
            }
        }

        let group = &comment_lines[group_start..group_end];
        let all_empty_or_border = group
            .iter()
            .all(|comment| comment.is_empty || (!allow_border && comment.is_border));

        if all_empty_or_border {
            for &comment in group {
                if comment.is_empty || (!allow_border && comment.is_border) {
                    push_comment_diagnostic(
                        source,
                        cop,
                        comment,
                        total_len,
                        diagnostics,
                        corrections.as_deref_mut(),
                    );
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
    parse_result: &ruby_prism::ParseResult<'_>,
    cop: &EmptyComment,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
) {
    let total_len = source.as_bytes().len();

    for comment in collect_comment_lines(source, lines, parse_result) {
        if comment.is_empty || (!allow_border && comment.is_border) {
            push_comment_diagnostic(
                source,
                cop,
                comment,
                total_len,
                diagnostics,
                corrections.as_deref_mut(),
            );
        }
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
