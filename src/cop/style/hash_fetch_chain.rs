use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE, NIL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HashFetchChain;

impl HashFetchChain {
    fn is_nil_or_empty_hash(node: &ruby_prism::Node<'_>) -> bool {
        // nil literal
        if node.as_nil_node().is_some() {
            return true;
        }
        // {} (empty hash literal) — keyword_hash_node is not applicable here
        // because keyword hashes only appear in argument positions and cannot
        // be a fetch default value.
        if let Some(hash) = node.as_hash_node() {
            if hash.elements().iter().next().is_none() {
                return true;
            }
        }
        // Hash.new or ::Hash.new
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"new" && call.arguments().is_none() {
                if let Some(recv) = call.receiver() {
                    if recv
                        .as_constant_read_node()
                        .is_some_and(|c| c.name().as_slice() == b"Hash")
                    {
                        return true;
                    }
                    if recv.as_constant_path_node().is_some_and(|cp| {
                        cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"Hash")
                    }) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl Cop for HashFetchChain {
    fn name(&self) -> &'static str {
        "Style/HashFetchChain"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            HASH_NODE,
            NIL_NODE,
        ]
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

        // Must be fetch method
        if call.name().as_slice() != b"fetch" {
            return;
        }

        // Must have 2 arguments (key, default)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 2 {
            return;
        }

        // The last fetch's default must be nil
        if !arg_list[1].as_nil_node().is_some() {
            return;
        }

        // The receiver must also be a fetch call with nil/{}/Hash.new default
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if recv_call.name().as_slice() != b"fetch" {
            return;
        }

        let recv_args = match recv_call.arguments() {
            Some(a) => a,
            None => return,
        };
        let recv_arg_list: Vec<_> = recv_args.arguments().iter().collect();
        if recv_arg_list.len() != 2 {
            return;
        }

        if !Self::is_nil_or_empty_hash(&recv_arg_list[1]) {
            return;
        }

        // Must not have a block
        if call.block().is_some() || recv_call.block().is_some() {
            return;
        }

        // Build dig arguments
        let first_key_src = &source.as_bytes()
            [recv_arg_list[0].location().start_offset()..recv_arg_list[0].location().end_offset()];
        let second_key_src = &source.as_bytes()
            [arg_list[0].location().start_offset()..arg_list[0].location().end_offset()];
        let first_key = String::from_utf8_lossy(first_key_src);
        let second_key = String::from_utf8_lossy(second_key_src);

        let msg_loc = recv_call
            .message_loc()
            .unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `dig({}, {})` instead.", first_key, second_key),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashFetchChain, "cops/style/hash_fetch_chain");
}
