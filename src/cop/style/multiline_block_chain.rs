use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineBlockChain;

impl Cop for MultilineBlockChain {
    fn name(&self) -> &'static str {
        "Style/MultilineBlockChain"
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

        // Check if the receiver is a multiline block
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // The receiver must be a call with a block
        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have a block
        if recv_call.block().is_none() {
            return Vec::new();
        }

        // The receiver (block) must be multiline
        let recv_loc = receiver.location();
        let (recv_start, _) = source.offset_to_line_col(recv_loc.start_offset());
        let (recv_end, _) =
            source.offset_to_line_col(recv_loc.end_offset().saturating_sub(1));

        if recv_start == recv_end {
            return Vec::new();
        }

        // This is the call chained on a multiline block
        let msg_loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid multi-line chains of blocks.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineBlockChain, "cops/style/multiline_block_chain");
}
