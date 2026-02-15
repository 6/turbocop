use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineMethodCallIndentation;

impl Cop for MultilineMethodCallIndentation {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodCallIndentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have a receiver (chained call)
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Must have a call operator (the `.` part)
        let dot_loc = match call_node.call_operator_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let receiver_loc = receiver.location();
        let (recv_end_line, _) = source.offset_to_line_col(receiver_loc.end_offset());
        let (msg_line, msg_col) = source.offset_to_line_col(dot_loc.start_offset());

        // Only check when the dot is on a different line than the end of its receiver.
        // This handles `foo(\n  ...\n).bar` where `.bar` is on the same line as `)`.
        if msg_line == recv_end_line {
            return Vec::new();
        }

        let style = config.get_str("EnforcedStyle", "aligned");
        let width = config.get_usize("IndentationWidth", 2);

        let expected = match style {
            "indented" => {
                let chain_start_line = find_chain_start_line(source, &receiver);
                let chain_line_bytes =
                    source.lines().nth(chain_start_line - 1).unwrap_or(b"");
                indentation_of(chain_line_bytes) + width
            }
            "indented_relative_to_receiver" => {
                let chain_start_line = find_chain_start_line(source, &receiver);
                let chain_line_bytes =
                    source.lines().nth(chain_start_line - 1).unwrap_or(b"");
                indentation_of(chain_line_bytes) + width
            }
            _ => {
                // "aligned" (default): dot should align with the first dot in the chain
                match find_alignment_dot_col(source, &receiver, msg_line) {
                    Some(col) => col,
                    // First multiline step — no previous dot to align with;
                    // accept whatever position the user chose
                    None => return Vec::new(),
                }
            }
        };

        if msg_col != expected {
            let msg = match style {
                "aligned" => {
                    "Align `.` with `.` on the previous line of the chain.".to_string()
                }
                _ => {
                    let chain_start_line = find_chain_start_line(source, &receiver);
                    let chain_line_bytes =
                        source.lines().nth(chain_start_line - 1).unwrap_or(b"");
                    let chain_indent = indentation_of(chain_line_bytes);
                    format!(
                        "Use {} (not {}) spaces for indentation of a chained method call.",
                        width,
                        msg_col.saturating_sub(chain_indent)
                    )
                }
            };
            return vec![self.diagnostic(source, msg_line, msg_col, msg)];
        }

        Vec::new()
    }
}

/// For `aligned` style: find the column of the dot (call operator) from the
/// nearest ancestor in the chain that is on a previous line.
fn find_alignment_dot_col(
    source: &SourceFile,
    receiver: &ruby_prism::Node<'_>,
    current_dot_line: usize,
) -> Option<usize> {
    // Walk up the receiver chain looking for a dot on a different (earlier) line
    if let Some(call) = receiver.as_call_node() {
        if let Some(dot_loc) = call.call_operator_loc() {
            let (dot_line, dot_col) = source.offset_to_line_col(dot_loc.start_offset());
            if dot_line < current_dot_line {
                // Found a dot on a previous line — this is the alignment target.
                // But check if there's an even earlier one (to get the first dot).
                if let Some(recv) = call.receiver() {
                    if let Some(earlier) = find_alignment_dot_col(source, &recv, dot_line)
                    {
                        return Some(earlier);
                    }
                }
                return Some(dot_col);
            }
            // Dot is on the same line as current; keep looking up
            if let Some(recv) = call.receiver() {
                return find_alignment_dot_col(source, &recv, current_dot_line);
            }
        }
    }
    None
}

fn find_chain_start_line(source: &SourceFile, node: &ruby_prism::Node<'_>) -> usize {
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            let (recv_line, _) = source.offset_to_line_col(recv.location().start_offset());
            let (call_msg_line, _) = if let Some(dot_loc) = call.call_operator_loc() {
                source.offset_to_line_col(dot_loc.start_offset())
            } else {
                (recv_line, 0)
            };
            // If this call is also multiline chained, recurse
            if call_msg_line != recv_line {
                return find_chain_start_line(source, &recv);
            }
        }
    }
    let (line, _) = source.offset_to_line_col(node.location().start_offset());
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        MultilineMethodCallIndentation,
        "cops/layout/multiline_method_call_indentation"
    );

    #[test]
    fn same_line_chain_ignored() {
        let source = b"foo.bar.baz\n";
        let diags = run_cop_full(&MultilineMethodCallIndentation, source);
        assert!(diags.is_empty());
    }
}
