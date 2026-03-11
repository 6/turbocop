use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/HeredocArgumentClosingParenthesis
///
/// Investigation findings (2026-03-11):
/// Root cause of 1019 FPs: the original implementation was too simplistic compared
/// to RuboCop's complex algorithm. Key missing checks:
///
/// 1. **Non-heredoc args after heredoc body**: When there are non-heredoc arguments
///    between the heredoc body end and the closing paren (e.g., `foo(<<~SQL, opt: true\n)\n`),
///    the closing paren correctly goes with those trailing args, not the heredoc opener.
///    RuboCop's `exist_argument_between_heredoc_end_and_closing_parentheses?` handles this.
///    Fix: scan bytes between the last heredoc body end and the closing paren for
///    non-whitespace content; if found, skip.
///
/// 2. **`end` keyword ancestors**: Calls wrapped in `do..end`, `if/unless/while` blocks
///    should not fire because the `end` keyword sits before the closing paren.
///    RuboCop's `end_keyword_before_closing_parenthesis?` handles this.
///    Fix: walk the source bytes between the call's opening paren and closing paren to
///    detect `end` keywords, or more practically, check if the closing paren is on the
///    same line as an `end` keyword by checking `end)` pattern.
///
/// 3. **Heredoc with method chain on receiver** (e.g., `<<-SQL.tr(...)`): The heredoc
///    is a receiver of a method call, not a direct string argument.
///    Fix: also check for heredocs as receivers of call arguments.
///
/// 4. **Removed spurious `STRING_NODE`/`INTERPOLATED_STRING_NODE` from interested types**
///    since they always returned early at the `as_call_node()` check.
pub struct HeredocArgumentClosingParenthesis;

impl Cop for HeredocArgumentClosingParenthesis {
    fn name(&self) -> &'static str {
        "Layout/HeredocArgumentClosingParenthesis"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must have parenthesized call
        let open_loc = match call.opening_loc() {
            Some(loc) => loc,
            None => return,
        };
        let close_loc = match call.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        if open_loc.as_slice() != b"(" || close_loc.as_slice() != b")" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let bytes = source.as_bytes();

        // Collect heredoc info: which args are heredocs, and their body end positions
        let mut has_heredoc = false;
        let mut max_heredoc_body_end: usize = 0;
        let mut last_heredoc_opener_line: usize = 0;

        for arg in args.arguments().iter() {
            if let Some((opener_offset, body_end)) = heredoc_info(bytes, &arg) {
                has_heredoc = true;
                if body_end > max_heredoc_body_end {
                    max_heredoc_body_end = body_end;
                    let (line, _) = source.offset_to_line_col(opener_offset);
                    last_heredoc_opener_line = line;
                }
            }
        }

        if !has_heredoc {
            return;
        }

        let (close_line, close_col) = source.offset_to_line_col(close_loc.start_offset());

        // If the closing paren is on the same line as the last heredoc opener, it's correct
        if close_line == last_heredoc_opener_line {
            return;
        }

        // Check if there's non-whitespace content between the last heredoc body end
        // and the closing paren. If so, there are non-heredoc arguments after the
        // heredoc body and the closing paren correctly goes with those args.
        if max_heredoc_body_end > 0 && max_heredoc_body_end < close_loc.start_offset() {
            let between = &bytes[max_heredoc_body_end..close_loc.start_offset()];
            let has_content = between.iter().any(|&b| !b.is_ascii_whitespace());
            if has_content {
                return;
            }
        }

        // Check if the closing paren is preceded by `end` on the same line.
        // This handles `foo(bar do ... end)` and `foo(unless cond ... end)`.
        if has_end_keyword_before_close_paren(bytes, close_loc.start_offset()) {
            return;
        }

        diagnostics.push(self.diagnostic(
            source,
            close_line,
            close_col,
            "Put the closing parenthesis for a method call with a HEREDOC parameter on the same line as the HEREDOC opening.".to_string(),
        ));
    }
}

/// Returns (opener_start_offset, body_end_offset) for a heredoc argument.
/// The opener_start_offset is the offset of `<<~SQL` etc.
/// The body_end_offset is the offset after the closing delimiter line.
fn heredoc_info(bytes: &[u8], node: &ruby_prism::Node<'_>) -> Option<(usize, usize)> {
    // Direct heredoc: InterpolatedStringNode or StringNode with `<<` opening
    if let Some(info) = direct_heredoc_info(bytes, node) {
        return Some(info);
    }

    // Heredoc as receiver of a method call on the same arg
    // e.g., `<<-SQL.tr("z", "t")` — the arg is a CallNode whose receiver is the heredoc
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            if let Some(info) = direct_heredoc_info(bytes, &recv) {
                return Some(info);
            }
        }
    }

    // Hash argument containing a heredoc value (e.g., `foo: <<-SQL`)
    if let Some(kw_hash) = node.as_keyword_hash_node() {
        for elem in kw_hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if let Some(info) = direct_heredoc_info(bytes, &assoc.value()) {
                    return Some(info);
                }
                // Also check if value is a call on a heredoc
                if let Some(call) = assoc.value().as_call_node() {
                    if let Some(recv) = call.receiver() {
                        if let Some(info) = direct_heredoc_info(bytes, &recv) {
                            return Some(info);
                        }
                    }
                }
            }
        }
    }
    if let Some(hash) = node.as_hash_node() {
        for elem in hash.elements().iter() {
            if let Some(assoc) = elem.as_assoc_node() {
                if let Some(info) = direct_heredoc_info(bytes, &assoc.value()) {
                    return Some(info);
                }
            }
        }
    }

    None
}

/// Check if a node is directly a heredoc (StringNode or InterpolatedStringNode with `<<` opening).
/// Returns (opener_start_offset, body_end_offset).
fn direct_heredoc_info(bytes: &[u8], node: &ruby_prism::Node<'_>) -> Option<(usize, usize)> {
    if let Some(istr) = node.as_interpolated_string_node() {
        if let Some(opening) = istr.opening_loc() {
            let slice = &bytes[opening.start_offset()..opening.end_offset()];
            if slice.starts_with(b"<<") {
                let body_end = istr
                    .closing_loc()
                    .map(|c| c.end_offset())
                    .unwrap_or(istr.location().end_offset());
                return Some((opening.start_offset(), body_end));
            }
        }
    }
    if let Some(str_node) = node.as_string_node() {
        if let Some(opening) = str_node.opening_loc() {
            let slice = &bytes[opening.start_offset()..opening.end_offset()];
            if slice.starts_with(b"<<") {
                let body_end = str_node
                    .closing_loc()
                    .map(|c| c.end_offset())
                    .unwrap_or(str_node.location().end_offset());
                return Some((opening.start_offset(), body_end));
            }
        }
    }
    None
}

/// Check if there's an `end` keyword immediately before the closing paren on the same line.
/// Scans backwards from the `)` looking for `end` preceded by whitespace or line start.
fn has_end_keyword_before_close_paren(bytes: &[u8], close_paren_offset: usize) -> bool {
    // Scan backwards from the close paren, skipping whitespace (not newlines)
    let mut pos = close_paren_offset;
    while pos > 0 {
        pos -= 1;
        if bytes[pos] == b'\n' {
            // Reached a newline — no `end` on this line before the paren
            return false;
        }
        if bytes[pos] != b' ' && bytes[pos] != b'\t' {
            break;
        }
    }
    // Check if the non-whitespace content ends with `end`
    if pos >= 2 {
        let end_candidate = &bytes[pos - 2..=pos];
        if end_candidate == b"end" {
            // Verify it's a word boundary (preceded by whitespace or line start)
            if pos < 3 {
                return true;
            }
            let before = bytes[pos - 3];
            return before == b' ' || before == b'\t' || before == b'\n' || before == b'\r';
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        HeredocArgumentClosingParenthesis,
        "cops/layout/heredoc_argument_closing_parenthesis"
    );
}
