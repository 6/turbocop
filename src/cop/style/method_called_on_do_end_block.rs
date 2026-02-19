use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};

pub struct MethodCalledOnDoEndBlock;

impl Cop for MethodCalledOnDoEndBlock {
    fn name(&self) -> &'static str {
        "Style/MethodCalledOnDoEndBlock"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE]
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

        // Skip if this call itself has a block (to avoid double-reporting with MultilineBlockChain)
        if call.block().is_some() {
            return Vec::new();
        }

        // Check if the receiver is a call with a do...end block
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have a block
        let block = match recv_call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Must be a do...end block (check opening_loc is "do")
        let opening_loc = block_node.opening_loc();
        if opening_loc.as_slice() != b"do" {
            return Vec::new();
        }

        let msg_loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid chaining a method call on a do...end block.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MethodCalledOnDoEndBlock, "cops/style/method_called_on_do_end_block");
}
