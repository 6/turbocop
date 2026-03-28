use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/SpaceBeforeFirstArg checks for extra space between a method name
/// and the first argument in calls without parentheses.
///
/// ## Investigation findings (2026-03-23)
///
/// The original implementation had a 15% match rate (95 matches, 537 FNs)
/// because it treated `AllowForAlignment: true` as unconditionally allowing
/// any extra space. RuboCop's behavior is more nuanced: it only allows
/// extra space when the first argument's column is actually aligned with
/// a token boundary on an adjacent line (using `aligned_with_something?`
/// from `PrecedingFollowingAlignment`). The fix implements alignment
/// checking: look at the preceding and following non-blank lines and
/// verify that the argument column has a `\s\S` boundary (space followed
/// by non-space) at the same position, indicating intentional alignment.
///
/// ## Investigation findings (2026-03-24)
///
/// FP=69, FN=124 from corpus. Two issues found:
/// 1. Tab characters in the gap between method name and first argument were
///    not being flagged (the check required all-spaces). Fixed to accept
///    tabs as whitespace in the gap.
/// 2. Alignment check was checking up to 2 nearest non-blank lines per
///    direction, while RuboCop uses a two-pass approach: pass 1 checks only
///    the nearest non-blank line, pass 2 checks the nearest line with the
///    same base indentation. The old approach could miss alignment when the
///    aligned line was separated by differently-indented lines (FPs), and
///    could falsely detect alignment from a 2nd non-blank line that RuboCop
///    wouldn't consider (FNs).
///
/// ## Investigation findings (2026-03-28)
///
/// FN=19 from corpus. The fallback alignment check was comparing only a short
/// token fragment from the first argument (for example `@`, `Token`, or `&`)
/// instead of the full first-argument source. That caused false alignment on
/// lines like `assert  @gateway...` versus `assert !@gateway...`, which share a
/// prefix at the same column but are not aligned by RuboCop's
/// `aligned_words?` check. Fixed by comparing the full first-argument source,
/// matching RuboCop's `range.source` behavior.
pub struct SpaceBeforeFirstArg;

const OPERATOR_METHODS: &[&[u8]] = &[
    b"+", b"-", b"*", b"/", b"**", b"%", b"==", b"!=", b"<", b">", b"<=", b">=", b"<=>", b"===",
    b"=~", b"!~", b"&", b"|", b"^", b"~", b"<<", b">>", b"[]", b"[]=", b"+@", b"-@",
];

fn is_operator_method(name: &[u8]) -> bool {
    OPERATOR_METHODS.contains(&name)
}

fn is_setter_method(name: &[u8]) -> bool {
    // Setter methods end with `=` but are not comparison operators
    name.len() >= 2 && name.last() == Some(&b'=') && !is_operator_method(name)
}

/// Check if the argument at `arg_col` (0-indexed byte column) is aligned with
/// a token boundary on an adjacent line. Mirrors RuboCop's `aligned_with_something?`
/// from `PrecedingFollowingAlignment`.
///
/// Uses a two-pass approach matching RuboCop's `aligned_with_any_line_range?`:
/// - Pass 1: Check the nearest non-blank, non-comment line in each direction
/// - Pass 2: Check the nearest non-blank, non-comment line with the same
///   base indentation in each direction (may look further to find it)
///
/// Alignment is detected by:
/// - Mode 1: space-then-non-space at `arg_col - 1` (token boundary alignment)
/// - Mode 2: exact first-argument source match at `arg_col`
fn is_aligned_with_adjacent(
    source: &SourceFile,
    line: usize,
    arg_col: usize,
    current_arg: &[u8],
) -> bool {
    let lines: Vec<&[u8]> = source.lines().collect();
    let current_line_idx = line - 1; // Convert 1-indexed to 0-indexed
    let current_line = lines.get(current_line_idx).copied().unwrap_or(&[]);

    // Pass 1: check the nearest non-blank, non-comment line in each direction.
    // RuboCop's aligned_with_line? yields the first qualifying line and returns.
    if let Some(adj) = find_nearest_nonblank(&lines, current_line_idx, Direction::Up, None) {
        if check_alignment_at(adj, arg_col, current_arg) {
            return true;
        }
    }
    if let Some(adj) = find_nearest_nonblank(&lines, current_line_idx, Direction::Down, None) {
        if check_alignment_at(adj, arg_col, current_arg) {
            return true;
        }
    }

    // Pass 2: check the nearest line with the same base indentation.
    let base_indent = line_indentation(current_line);
    if let Some(adj) =
        find_nearest_nonblank(&lines, current_line_idx, Direction::Up, Some(base_indent))
    {
        if check_alignment_at(adj, arg_col, current_arg) {
            return true;
        }
    }
    if let Some(adj) =
        find_nearest_nonblank(&lines, current_line_idx, Direction::Down, Some(base_indent))
    {
        if check_alignment_at(adj, arg_col, current_arg) {
            return true;
        }
    }

    false
}

enum Direction {
    Up,
    Down,
}

/// Find the nearest non-blank, non-comment line in the given direction.
/// If `required_indent` is Some, only consider lines with that exact indentation.
fn find_nearest_nonblank<'a>(
    lines: &[&'a [u8]],
    current_idx: usize,
    direction: Direction,
    required_indent: Option<usize>,
) -> Option<&'a [u8]> {
    let mut idx = current_idx;
    loop {
        match direction {
            Direction::Up => {
                if idx == 0 {
                    return None;
                }
                idx -= 1;
            }
            Direction::Down => {
                if idx + 1 >= lines.len() {
                    return None;
                }
                idx += 1;
            }
        }
        let line = lines[idx];
        if is_blank_or_comment(line) {
            continue;
        }
        if let Some(indent) = required_indent {
            if line_indentation(line) != indent {
                continue;
            }
        }
        return Some(line);
    }
}

/// Compute the indentation level (number of leading spaces/tabs) of a line.
fn line_indentation(line: &[u8]) -> usize {
    line.iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count()
}

/// Check if there's a token boundary at `col` on the given line,
/// mirroring RuboCop's `aligned_words?`.
fn check_alignment_at(adj_line: &[u8], col: usize, current_arg: &[u8]) -> bool {
    if col >= adj_line.len() {
        return false;
    }

    // Mode 1: space + non-space at the same column (token boundary)
    if adj_line[col] != b' '
        && adj_line[col] != b'\t'
        && col > 0
        && (adj_line[col - 1] == b' ' || adj_line[col - 1] == b'\t')
    {
        return true;
    }

    // Mode 2: exact first-argument source match at the same position
    if !current_arg.is_empty()
        && col + current_arg.len() <= adj_line.len()
        && &adj_line[col..col + current_arg.len()] == current_arg
    {
        return true;
    }

    false
}

/// Check if a line is blank or a comment-only line.
fn is_blank_or_comment(line: &[u8]) -> bool {
    let trimmed = line.iter().skip_while(|&&b| b == b' ' || b == b'\t');
    match trimmed.clone().next() {
        None => true,        // blank line
        Some(&b'#') => true, // comment line
        _ => false,
    }
}

impl Cop for SpaceBeforeFirstArg {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeFirstArg"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_for_alignment = config.get_bool("AllowForAlignment", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Only check calls without parentheses
        if call.opening_loc().is_some() {
            return;
        }

        // Must have regular arguments or a block-pass argument (`&arg`)
        let (arg_start, arg_end) = if let Some(args) = call.arguments() {
            match args.arguments().iter().next() {
                Some(arg) => {
                    let loc = arg.location();
                    (loc.start_offset(), loc.end_offset())
                }
                None => return,
            }
        } else if let Some(block_arg) = call.block().and_then(|b| b.as_block_argument_node()) {
            let loc = block_arg.location();
            (loc.start_offset(), loc.end_offset())
        } else {
            return;
        };

        // Skip operator methods (e.g. `2**128`, `x + 1`) and setter methods (e.g. `self.foo=`)
        // These are parsed as CallNodes but should not be checked.
        let method_name = call.name();
        let name_bytes = method_name.as_slice();
        if is_operator_method(name_bytes) || is_setter_method(name_bytes) {
            return;
        }

        // Get the method name location
        let msg_loc = call.message_loc();
        let msg_loc = match msg_loc {
            Some(l) => l,
            None => return,
        };

        let method_end = msg_loc.end_offset();

        // Must be on the same line
        let (method_line, _) = source.offset_to_line_col(method_end);
        let (arg_line, _) = source.offset_to_line_col(arg_start);
        if method_line != arg_line {
            return;
        }

        let gap = arg_start.saturating_sub(method_end);

        if gap == 0 {
            // No space at all between method name and first arg — always flag
            let (line, column) = source.offset_to_line_col(method_end);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Put one space between the method name and the first argument.".to_string(),
            ));
        }

        if gap > 1 {
            // More than one space/tab between method name and first arg
            let bytes = source.as_bytes();
            let between = &bytes[method_end..arg_start];
            if between.iter().all(|&b| b == b' ' || b == b'\t') {
                // When AllowForAlignment is true (default), check if the argument
                // is actually aligned with a token on an adjacent line.
                if allow_for_alignment {
                    let current_arg = &bytes[arg_start..arg_end];
                    // Compute the byte column of the first argument on its line
                    let line_start = source.line_start_offset(method_line);
                    let arg_byte_col = arg_start - line_start;
                    if is_aligned_with_adjacent(source, method_line, arg_byte_col, current_arg) {
                        return;
                    }
                }

                let (line, column) = source.offset_to_line_col(method_end);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Put one space between the method name and the first argument.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeFirstArg, "cops/layout/space_before_first_arg");
}
