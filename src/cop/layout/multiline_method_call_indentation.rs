use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
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
        let (recv_line, _) = source.offset_to_line_col(receiver_loc.start_offset());
        let (msg_line, msg_col) = source.offset_to_line_col(dot_loc.start_offset());

        // Only check multiline chained calls
        if msg_line == recv_line {
            return Vec::new();
        }

        let width = config
            .options
            .get("IndentationWidth")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        // Find the start of the chain: walk up receivers to find the first receiver
        // that starts on a different line or has no receiver itself
        let chain_start_line = find_chain_start_line(source, &receiver);
        let chain_line_bytes = source.lines().nth(chain_start_line - 1).unwrap_or(b"");
        let chain_indent = indentation_of(chain_line_bytes);
        let expected = chain_indent + width;

        // Account for the dot: msg_col points at `.`, so the indent should
        // be measured from the dot position
        if msg_col != expected {
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location {
                    line: msg_line,
                    column: msg_col,
                },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: format!(
                    "Use {} (not {}) spaces for indentation of a chained method call.",
                    width,
                    msg_col.saturating_sub(chain_indent)
                ),
            }];
        }

        Vec::new()
    }
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
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &MultilineMethodCallIndentation,
            include_bytes!(
                "../../../testdata/cops/layout/multiline_method_call_indentation/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &MultilineMethodCallIndentation,
            include_bytes!(
                "../../../testdata/cops/layout/multiline_method_call_indentation/no_offense.rb"
            ),
        );
    }

    #[test]
    fn same_line_chain_ignored() {
        let source = b"foo.bar.baz\n";
        let diags = run_cop_full(&MultilineMethodCallIndentation, source);
        assert!(diags.is_empty());
    }
}
