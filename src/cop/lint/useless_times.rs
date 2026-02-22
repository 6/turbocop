use crate::cop::node_type::{CALL_NODE, INTEGER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UselessTimes;

impl Cop for UselessTimes {
    fn name(&self) -> &'static str {
        "Lint/UselessTimes"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE]
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
        // Look for `N.times` where N is 0, 1, or negative
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"times" {
            return;
        }

        // Must have no arguments (times takes no args)
        if call.arguments().is_some() {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check if the receiver is an integer literal 0 or 1
        let recv_value = get_integer_value(&receiver, source);
        let value = match recv_value {
            Some(v) => v,
            None => return,
        };

        if value > 1 {
            return;
        }

        // Get the display text for the number
        let recv_text = std::str::from_utf8(
            &source.as_bytes()
                [receiver.location().start_offset()..receiver.location().end_offset()],
        )
        .unwrap_or("N");

        // Report on the call up to the `.times` part (excluding any block or chained methods)
        // Find the end of `.times`
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return,
        };

        let start = call.location().start_offset();
        let _end = msg_loc.end_offset();
        let (line, column) = source.offset_to_line_col(start);

        // If the call has a block, include it in the range
        let report_end = call.location().end_offset();

        // Use the full call range for the offense
        let _full_src = &source.as_bytes()[start..report_end];

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Useless call to `{}.times` detected.", recv_text),
        ));
    }
}

/// Extract the integer value from a node (handling negatives).
fn get_integer_value(node: &ruby_prism::Node<'_>, source: &SourceFile) -> Option<i64> {
    if let Some(int_node) = node.as_integer_node() {
        let src = &source.as_bytes()
            [int_node.location().start_offset()..int_node.location().end_offset()];
        let s = std::str::from_utf8(src).ok()?;
        return s.parse::<i64>().ok();
    }
    // Handle unary minus: -1
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"-@" {
            if let Some(recv) = call.receiver() {
                if let Some(int_node) = recv.as_integer_node() {
                    let src = &source.as_bytes()
                        [int_node.location().start_offset()..int_node.location().end_offset()];
                    let s = std::str::from_utf8(src).ok()?;
                    let v = s.parse::<i64>().ok()?;
                    return Some(-v);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessTimes, "cops/lint/useless_times");
}
