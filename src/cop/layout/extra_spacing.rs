use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;
use std::ops::Range;

pub struct ExtraSpacing;

impl Cop for ExtraSpacing {
    fn name(&self) -> &'static str {
        "Layout/ExtraSpacing"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_for_alignment = config.get_bool("AllowForAlignment", true);
        let allow_before_trailing_comments =
            config.get_bool("AllowBeforeTrailingComments", false);
        let _force_equal_sign_alignment = config.get_bool("ForceEqualSignAlignment", false);

        let lines: Vec<&[u8]> = source.lines().collect();
        let src_bytes = source.as_bytes();

        // Collect multiline hash pair ranges to ignore (key..value spacing
        // is handled by Layout/HashAlignment, not this cop).
        let ignored_ranges = collect_hash_pair_ranges(parse_result, src_bytes);

        // Build the set of aligned comment lines (1-indexed). Two consecutive
        // comments that start at the same column are both considered "aligned".
        let aligned_comment_lines = build_aligned_comment_lines(parse_result, source);

        // Identify comment-only lines (0-indexed) for skipping during alignment search
        let comment_only_lines = build_comment_only_lines(&lines);

        let mut diagnostics = Vec::new();

        // Track cumulative byte offset for each line start
        let mut line_start_offset: usize = 0;

        for (line_idx, &line) in lines.iter().enumerate() {
            let line_num = line_idx + 1;
            let mut i = 0;

            // Skip leading whitespace (indentation)
            while i < line.len() && (line[i] == b' ' || line[i] == b'\t') {
                i += 1;
            }

            // Now scan for extra spaces within the line
            while i < line.len() {
                if line[i] == b' ' {
                    let space_start = i;
                    while i < line.len() && line[i] == b' ' {
                        i += 1;
                    }
                    let space_count = i - space_start;

                    if space_count > 1 && i < line.len() {
                        // Get the byte offset in the full source
                        let abs_offset = line_start_offset + space_start;

                        // Skip if inside string/comment
                        if !code_map.is_code(abs_offset) {
                            continue;
                        }

                        // Skip if inside a multiline hash pair (key => value
                        // or key: value) -- handled by Layout/HashAlignment
                        if is_in_ignored_range(&ignored_ranges, abs_offset) {
                            continue;
                        }

                        // Skip if before trailing comment and that's allowed
                        if allow_before_trailing_comments && line[i] == b'#' {
                            continue;
                        }

                        // For trailing comments: check if the comment is aligned
                        // with other comments (RuboCop's aligned_comments logic)
                        if allow_for_alignment
                            && line[i] == b'#'
                            && aligned_comment_lines.contains(&line_num)
                        {
                            continue;
                        }

                        // Skip if this could be alignment with adjacent code
                        if allow_for_alignment
                            && is_aligned_with_adjacent(
                                &lines,
                                line_idx,
                                i,
                                &comment_only_lines,
                            )
                        {
                            continue;
                        }

                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            space_start + 1, // point to the first extra space
                            "Unnecessary spacing detected.".to_string(),
                        ));
                    }
                } else {
                    i += 1;
                }
            }

            // Advance to next line: line content + 1 for '\n'
            line_start_offset += line.len() + 1;
        }

        diagnostics
    }
}

// -- Multiline hash pair ignored ranges --

/// Collect byte ranges between keys and values in multiline hash pairs.
fn collect_hash_pair_ranges(
    parse_result: &ruby_prism::ParseResult<'_>,
    src_bytes: &[u8],
) -> Vec<Range<usize>> {
    let mut collector = HashPairCollector {
        ranges: Vec::new(),
        src_bytes,
    };
    collector.visit(&parse_result.node());
    collector.ranges
}

struct HashPairCollector<'a> {
    ranges: Vec<Range<usize>>,
    src_bytes: &'a [u8],
}

impl<'pr> Visit<'pr> for HashPairCollector<'_> {
    fn visit_hash_node(&mut self, node: &ruby_prism::HashNode<'pr>) {
        self.collect_multiline_pairs(node.elements().iter(), &node.location());
        ruby_prism::visit_hash_node(self, node);
    }

    fn visit_keyword_hash_node(&mut self, node: &ruby_prism::KeywordHashNode<'pr>) {
        self.collect_multiline_pairs(node.elements().iter(), &node.location());
        ruby_prism::visit_keyword_hash_node(self, node);
    }
}

impl HashPairCollector<'_> {
    fn collect_multiline_pairs<'a>(
        &mut self,
        elements: impl Iterator<Item = ruby_prism::Node<'a>>,
        parent_loc: &ruby_prism::Location<'_>,
    ) {
        let start = parent_loc.start_offset();
        let end = parent_loc.end_offset().min(self.src_bytes.len());
        let is_multiline = self.src_bytes[start..end].contains(&b'\n');
        if !is_multiline {
            return;
        }
        for element in elements {
            if let Some(assoc) = element.as_assoc_node() {
                let key_end = assoc.key().location().end_offset();
                let val_start = assoc.value().location().start_offset();
                if val_start > key_end {
                    self.ranges.push(key_end..val_start);
                }
            }
        }
    }
}

fn is_in_ignored_range(ranges: &[Range<usize>], offset: usize) -> bool {
    ranges.iter().any(|r| r.contains(&offset))
}

// -- Aligned comments --

/// Build a set of line numbers (1-indexed) where trailing comments are
/// aligned with adjacent comments at the same column.
fn build_aligned_comment_lines(
    parse_result: &ruby_prism::ParseResult<'_>,
    source: &SourceFile,
) -> HashSet<usize> {
    let mut comment_locs: Vec<(usize, usize)> = Vec::new();
    for comment in parse_result.comments() {
        let loc = comment.location();
        let (line, col) = source.offset_to_line_col(loc.start_offset());
        comment_locs.push((line, col));
    }
    comment_locs.sort_unstable();

    let mut aligned = HashSet::new();
    for pair in comment_locs.windows(2) {
        let (line1, col1) = pair[0];
        let (line2, col2) = pair[1];
        if col1 == col2 {
            aligned.insert(line1);
            aligned.insert(line2);
        }
    }
    aligned
}

// -- Comment-only lines --

fn build_comment_only_lines(lines: &[&[u8]]) -> HashSet<usize> {
    let mut set = HashSet::new();
    for (idx, line) in lines.iter().enumerate() {
        let first_non_ws = line.iter().position(|&b| b != b' ' && b != b'\t');
        if let Some(pos) = first_non_ws {
            if line[pos] == b'#' {
                set.insert(idx);
            }
        }
    }
    set
}

// -- Alignment detection --

/// Check if the token at `col` aligns with a token on a nearby line.
///
/// Implements RuboCop's PrecedingFollowingAlignment:
/// 1. First pass: nearest non-blank, non-comment-only line in each direction.
/// 2. Second pass: nearest line with the same indentation in each direction.
fn is_aligned_with_adjacent(
    lines: &[&[u8]],
    line_idx: usize,
    col: usize,
    comment_only_lines: &HashSet<usize>,
) -> bool {
    let base_indent = line_indentation(lines[line_idx]);
    let token_char = lines[line_idx][col];

    let current_line = lines[line_idx];

    // Pass 1: nearest non-blank, non-comment-only line
    if let Some(adj) =
        find_nearest_line(lines, line_idx, true, comment_only_lines, None)
    {
        if check_alignment(lines[adj], col, token_char)
            || check_equals_alignment(current_line, lines[adj], col)
        {
            return true;
        }
    }
    if let Some(adj) =
        find_nearest_line(lines, line_idx, false, comment_only_lines, None)
    {
        if check_alignment(lines[adj], col, token_char)
            || check_equals_alignment(current_line, lines[adj], col)
        {
            return true;
        }
    }

    // Pass 2: nearest line with same indentation
    if let Some(adj) =
        find_nearest_line(lines, line_idx, true, comment_only_lines, Some(base_indent))
    {
        if check_alignment(lines[adj], col, token_char)
            || check_equals_alignment(current_line, lines[adj], col)
        {
            return true;
        }
    }
    if let Some(adj) = find_nearest_line(
        lines,
        line_idx,
        false,
        comment_only_lines,
        Some(base_indent),
    ) {
        if check_alignment(lines[adj], col, token_char)
            || check_equals_alignment(current_line, lines[adj], col)
        {
            return true;
        }
    }

    false
}

/// Find the nearest non-blank, non-comment-only line in the given direction.
/// When `required_indent` is None, returns the very first non-blank, non-comment line.
/// When `required_indent` is Some, skips lines with different indentation (matching
/// RuboCop's PrecedingFollowingAlignment behavior which walks through all lines).
fn find_nearest_line(
    lines: &[&[u8]],
    start_idx: usize,
    going_up: bool,
    comment_only_lines: &HashSet<usize>,
    required_indent: Option<usize>,
) -> Option<usize> {
    let mut idx = start_idx;
    loop {
        if going_up {
            if idx == 0 {
                return None;
            }
            idx -= 1;
        } else {
            idx += 1;
            if idx >= lines.len() {
                return None;
            }
        }

        if comment_only_lines.contains(&idx) {
            continue;
        }

        let line = lines[idx];

        if line
            .iter()
            .all(|&b| b == b' ' || b == b'\t' || b == b'\r')
        {
            continue;
        }

        if let Some(indent) = required_indent {
            let this_indent = line_indentation(line);
            if this_indent != indent {
                continue;
            }
        }

        return Some(idx);
    }
}

/// Check alignment: either space+non-space at the column, the same character
/// at the column, or equals-sign alignment (e.g., '+=' aligns with '=').
fn check_alignment(line: &[u8], col: usize, token_char: u8) -> bool {
    if col >= line.len() {
        return false;
    }
    // Mode 1: space + non-space at the same column
    if line[col] != b' ' && line[col] != b'\t' {
        if col > 0 && (line[col - 1] == b' ' || line[col - 1] == b'\t') {
            return true;
        }
    }
    // Mode 2: same character at the same column
    if line[col] == token_char {
        return true;
    }
    false
}

/// Check if there's equals-sign alignment between the current line and
/// the adjacent line. For compound assignment operators like +=, -=, ||=,
/// &&=, the '=' sign should align with a '=' on the adjacent line.
fn check_equals_alignment(
    current_line: &[u8],
    adj_line: &[u8],
    col: usize,
) -> bool {
    // Find the '=' in or near the token starting at col on the current line
    let eq_col = find_equals_col(current_line, col);
    if let Some(eq_col) = eq_col {
        // Check if the adjacent line has '=' at the same column
        if eq_col < adj_line.len() && adj_line[eq_col] == b'=' {
            return true;
        }
    }
    false
}

/// Find the column of the '=' sign in an assignment operator starting at col.
/// Handles: =, ==, ===, !=, <=, >=, +=, -=, *=, /=, %=, **=, ||=, &&=, <<=, >>=
fn find_equals_col(line: &[u8], col: usize) -> Option<usize> {
    for offset in 0..4 {
        let c = col + offset;
        if c >= line.len() {
            break;
        }
        if line[c] == b'=' {
            return Some(c);
        }
        // Stop if we hit a space (we've gone past the token)
        if line[c] == b' ' || line[c] == b'\t' {
            break;
        }
    }
    None
}

fn line_indentation(line: &[u8]) -> usize {
    line.iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ExtraSpacing, "cops/layout/extra_spacing");
}
