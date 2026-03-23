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
/// from `PrecedingFollowingAlignment`, simplified for this cop's needs.
///
/// Checks preceding and following non-blank, non-comment lines for:
/// - Mode 1: space-then-non-space at `arg_col - 1` (token boundary alignment)
/// - Mode 2: exact token text match at `arg_col`
fn is_aligned_with_adjacent(source: &SourceFile, line: usize, arg_col: usize) -> bool {
    let lines: Vec<&[u8]> = source.lines().collect();
    let current_line_idx = line - 1; // Convert 1-indexed to 0-indexed

    // Extract the token starting at arg_col on the current line for Mode 2
    let current_line = lines.get(current_line_idx).copied().unwrap_or(&[]);
    let current_token = extract_token_at(current_line, arg_col);

    // Check preceding lines (up to 2 non-blank, non-comment lines)
    let mut checked = 0;
    let mut idx = current_line_idx;
    while idx > 0 && checked < 2 {
        idx -= 1;
        let adj = lines[idx];
        if is_blank_or_comment(adj) {
            continue;
        }
        checked += 1;
        if check_alignment_at(adj, arg_col, current_token) {
            return true;
        }
    }

    // Check following lines (up to 2 non-blank, non-comment lines)
    checked = 0;
    idx = current_line_idx;
    while idx + 1 < lines.len() && checked < 2 {
        idx += 1;
        let adj = lines[idx];
        if is_blank_or_comment(adj) {
            continue;
        }
        checked += 1;
        if check_alignment_at(adj, arg_col, current_token) {
            return true;
        }
    }

    false
}

/// Check if there's a token boundary at `col` on the given line,
/// mirroring RuboCop's `aligned_words?`.
fn check_alignment_at(adj_line: &[u8], col: usize, current_token: &[u8]) -> bool {
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

    // Mode 2: exact token match at the same position
    if !current_token.is_empty()
        && col + current_token.len() <= adj_line.len()
        && &adj_line[col..col + current_token.len()] == current_token
    {
        return true;
    }

    false
}

/// Extract a token-like string starting at the given byte column.
fn extract_token_at(line: &[u8], col: usize) -> &[u8] {
    if col >= line.len() {
        return &[];
    }
    let ch = line[col];
    if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b':' {
        let end = line[col..]
            .iter()
            .position(|&b| !b.is_ascii_alphanumeric() && b != b'_' && b != b':')
            .map_or(line.len(), |p| col + p);
        &line[col..end]
    } else if ch == b'"' || ch == b'\'' {
        if let Some(close_pos) = line[col + 1..].iter().position(|&b| b == ch) {
            &line[col..col + 1 + close_pos + 1]
        } else {
            &line[col..col + 1]
        }
    } else if ch == b' ' || ch == b'\t' {
        &[]
    } else {
        &line[col..col + 1]
    }
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

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
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

        let first_arg = match args.arguments().iter().next() {
            Some(a) => a,
            None => return,
        };

        let method_end = msg_loc.end_offset();
        let arg_start = first_arg.location().start_offset();

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
            // More than one space between method name and first arg
            let bytes = source.as_bytes();
            let between = &bytes[method_end..arg_start];
            if between.iter().all(|&b| b == b' ') {
                // When AllowForAlignment is true (default), check if the argument
                // is actually aligned with a token on an adjacent line.
                if allow_for_alignment {
                    // Compute the byte column of the first argument on its line
                    let line_start = source.line_start_offset(method_line);
                    let arg_byte_col = arg_start - line_start;
                    if is_aligned_with_adjacent(source, method_line, arg_byte_col) {
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
