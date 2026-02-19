use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct DotPosition;

impl Cop for DotPosition {
    fn name(&self) -> &'static str {
        "Layout/DotPosition"
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
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "leading");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have a dot (regular `.` or safe navigation `&.`)
        let dot_loc = match call.call_operator_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        // Must have a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Must have a method name (message)
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let (dot_line, dot_col) = source.offset_to_line_col(dot_loc.start_offset());
        let (recv_line, _) = source.offset_to_line_col(receiver.location().end_offset().saturating_sub(1));
        let (msg_line, _) = source.offset_to_line_col(msg_loc.start_offset());

        // Single line call — no issue
        if recv_line == msg_line {
            return Vec::new();
        }

        // If there's a blank line between dot and selector, skip (could be reformatted oddly)
        if (msg_line as i64 - dot_line as i64).abs() > 1 || (dot_line as i64 - recv_line as i64).abs() > 1 {
            return Vec::new();
        }

        let dot_str = std::str::from_utf8(dot_loc.as_slice()).unwrap_or(".");

        match style {
            "trailing" => {
                // Dot should be on the same line as the receiver (trailing)
                if dot_line != recv_line {
                    return vec![self.diagnostic(
                        source,
                        dot_line,
                        dot_col,
                        format!(
                            "Place the `{}` on the previous line, together with the method call receiver.",
                            dot_str
                        ),
                    )];
                }
            }
            _ => {
                // "leading" (default): dot should be on the same line as the method name
                if dot_line != msg_line {
                    return vec![self.diagnostic(
                        source,
                        dot_line,
                        dot_col,
                        format!(
                            "Place the `{}` on the next line, together with the method name.",
                            dot_str
                        ),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(DotPosition, "cops/layout/dot_position");
}
