use crate::cop::util::is_blank_line;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct EmptyLinesAroundArguments;

impl Cop for EmptyLinesAroundArguments {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundArguments"
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
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if args_list.is_empty() {
            return Vec::new();
        }

        // Check if the entire send node is single-line (not just the args)
        let (call_start_line, _) = source.offset_to_line_col(call.location().start_offset());
        let call_end = call.location().end_offset().saturating_sub(1);
        let (call_end_line, _) = source.offset_to_line_col(call_end);
        if call_start_line == call_end_line {
            return Vec::new();
        }

        // Skip if receiver and method call are on different lines
        // (e.g., `foo.\n  bar(arg)`)
        if let Some(receiver) = call.receiver() {
            if let Some(msg_loc) = call.message_loc() {
                let recv_end = receiver.location().end_offset().saturating_sub(1);
                let (recv_end_line, _) = source.offset_to_line_col(recv_end);
                let (msg_line, _) = source.offset_to_line_col(msg_loc.start_offset());
                if recv_end_line != msg_line {
                    return Vec::new();
                }
            }
        }

        let lines: Vec<&[u8]> = source.lines().collect();
        let bytes = source.as_bytes();
        let mut diagnostics = Vec::new();

        // RuboCop's approach: for each argument's start position and the closing
        // paren, look backward through whitespace. If there are blank lines
        // immediately before that position, flag them.
        //
        // This avoids flagging blank lines INSIDE argument values (hashes,
        // arrays, blocks, heredocs, etc.) which are not "around" the arguments.

        // Check before each argument
        for arg in &args_list {
            check_blank_lines_before(
                source,
                bytes,
                &lines,
                arg.location().start_offset(),
                &mut diagnostics,
                self,
            );
        }

        // Check before closing paren (if present)
        if let Some(close_loc) = call.closing_loc() {
            if close_loc.as_slice() == b")" {
                check_blank_lines_before(
                    source,
                    bytes,
                    &lines,
                    close_loc.start_offset(),
                    &mut diagnostics,
                    self,
                );
            }
        }

        diagnostics
    }
}

/// Check if there are blank lines immediately before `offset` by scanning
/// backwards through whitespace. If the whitespace gap spans more than 1 line,
/// there are blank lines that should be flagged.
fn check_blank_lines_before(
    source: &SourceFile,
    bytes: &[u8],
    lines: &[&[u8]],
    offset: usize,
    diagnostics: &mut Vec<Diagnostic>,
    cop: &EmptyLinesAroundArguments,
) {
    // Find the start of the line containing `offset`
    let (target_line, _) = source.offset_to_line_col(offset);

    // Scan backwards from `offset` to find the first non-whitespace character
    let mut pos = offset;
    while pos > 0 {
        let b = bytes[pos - 1];
        if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
            pos -= 1;
        } else {
            break;
        }
    }

    // `pos` now points just past the last non-whitespace char before our target.
    // Find what line that's on.
    let prev_line = if pos == 0 {
        1
    } else {
        source.offset_to_line_col(pos.saturating_sub(1)).0
    };

    // If there's more than 1 line gap, there are blank lines between them
    if target_line > prev_line + 1 {
        // Flag each blank line in the gap
        for line_num in (prev_line + 1)..target_line {
            if line_num > 0 && line_num <= lines.len() && is_blank_line(lines[line_num - 1]) {
                diagnostics.push(cop.diagnostic(
                    source,
                    line_num,
                    0,
                    "Empty line detected around arguments.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAroundArguments,
        "cops/layout/empty_lines_around_arguments"
    );
}
