use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-30):
/// The cop only handled direct `each.each*` chains, so it missed two RuboCop
/// patterns: iterator calls on `each_*`/`reverse_each` receivers
/// (`each_slice.each_with_index`, `each_key.each_with_object`, `each.reverse_each`)
/// and the selector-based offense locations RuboCop reports for chained calls.
/// This implementation now mirrors those narrow shapes while still skipping
/// receivers/current calls that already consume a real block or block-pass.
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if !is_redundant_iterator_name(method_name) {
            return;
        }

        if let Some(recv_each) = receiver_call_without_block_consumption(&call)
            .filter(|recv| recv.name().as_slice() == b"each")
        {
            let start_offset = selector_start(&recv_each);
            let (line, column) = source.offset_to_line_col(start_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Remove redundant `each`.".to_string(),
            ));
            return;
        }

        let Some(message) = offense_message(method_name) else {
            return;
        };

        // RuboCop skips current calls that already consume a block-pass (`&:foo`, `&block`).
        if has_block_pass(&call) {
            return;
        }

        let receiver_is_redundant =
            receiver_call_without_block_consumption(&call).is_some_and(|recv| {
                let recv_name = recv.name().as_slice();
                recv_name == b"reverse_each"
                    || (method_name != b"each" && recv_name.starts_with(b"each_"))
            });

        if !receiver_is_redundant {
            return;
        }

        let msg_loc = call.message_loc().unwrap_or_else(|| call.location());
        let start_offset = if method_name == b"each" && is_followed_by_call_chain(source, &call) {
            msg_loc.start_offset()
        } else if method_name == b"each" {
            call.call_operator_loc()
                .map_or(msg_loc.start_offset(), |op| op.start_offset())
        } else {
            msg_loc.start_offset()
        };
        let (line, column) = source.offset_to_line_col(start_offset);
        diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
    }
}

fn offense_message(method_name: &[u8]) -> Option<&'static str> {
    match method_name {
        b"each" => Some("Remove redundant `each`."),
        b"each_with_index" => Some("Use `with_index` to remove redundant `each`."),
        b"each_with_object" => Some("Use `with_object` to remove redundant `each`."),
        _ => None,
    }
}

fn is_redundant_iterator_name(method_name: &[u8]) -> bool {
    method_name == b"each"
        || method_name == b"each_with_index"
        || method_name == b"each_with_object"
        || method_name == b"reverse_each"
}

fn has_regular_block(call: &ruby_prism::CallNode<'_>) -> bool {
    call.block()
        .is_some_and(|block| block.as_block_node().is_some())
}

fn has_block_pass(call: &ruby_prism::CallNode<'_>) -> bool {
    call.block()
        .is_some_and(|block| block.as_block_argument_node().is_some())
}

fn selector_start(call: &ruby_prism::CallNode<'_>) -> usize {
    call.message_loc()
        .unwrap_or_else(|| call.location())
        .start_offset()
}

fn receiver_call_without_block_consumption<'pr>(
    call: &ruby_prism::CallNode<'pr>,
) -> Option<ruby_prism::CallNode<'pr>> {
    let recv_call = call.receiver()?.as_call_node()?;
    if has_regular_block(&recv_call) || has_block_pass(&recv_call) {
        return None;
    }
    Some(recv_call)
}

fn is_followed_by_call_chain(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> bool {
    let bytes = source.as_bytes();
    let mut offset = call.location().end_offset();
    while offset < bytes.len() && bytes[offset].is_ascii_whitespace() {
        offset += 1;
    }
    offset < bytes.len()
        && (bytes[offset] == b'.'
            || (bytes[offset] == b'&' && bytes.get(offset + 1) == Some(&b'.')))
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantEach, "cops/style/redundant_each");
}
