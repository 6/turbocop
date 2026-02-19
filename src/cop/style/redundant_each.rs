use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct RedundantEach;

impl Cop for RedundantEach {
    fn name(&self) -> &'static str {
        "Style/RedundantEach"
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
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_bytes = call.name().as_slice();

        // Check for each.each, each.each_with_index, each.each_with_object
        let is_each_method = method_bytes == b"each"
            || method_bytes == b"each_with_index"
            || method_bytes == b"each_with_object";

        if !is_each_method {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if recv_call.name().as_slice() != b"each" {
            return;
        }

        // The inner each must have no block and no arguments
        if recv_call.block().is_some() || recv_call.arguments().is_some() {
            return;
        }

        // Must have a receiver (not bare `each`)
        if recv_call.receiver().is_none() {
            return;
        }

        let msg_loc = recv_call.message_loc().unwrap_or_else(|| recv_call.location());
        // Include the dot before each
        let dot_start = if let Some(op) = recv_call.call_operator_loc() {
            op.start_offset()
        } else {
            msg_loc.start_offset()
        };
        let (line, column) = source.offset_to_line_col(dot_start);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Remove redundant `each`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantEach, "cops/style/redundant_each");
}
