use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE, NIL_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation: the single FP+FN pair was at the same location in
/// geocoder — a 3-level `.fetch('a', {}).fetch('b', {}).fetch('c', nil)` chain.
/// The cop only handled 2-level chains, reporting on the middle `.fetch` instead
/// of the outermost one. Fixed by walking up the receiver chain from the terminal
/// `fetch(key, nil)` to collect all chained fetch keys, then reporting on the
/// outermost fetch with the full `dig(a, b, c)` suggestion.
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

        // The terminal fetch's default must be nil
        if arg_list[1].as_nil_node().is_none() {
            return;
        }

        // Must not have a block
        if call.block().is_some() {
            return;
        }

        // Walk up the receiver chain collecting fetch(key, {}/nil/Hash.new) calls.
        // key_ranges stores (start_offset, end_offset) of each key arg, innermost first.
        // outermost_msg_offset tracks the message_loc of the outermost fetch.
        let key_loc = arg_list[0].location();
        let mut key_ranges = vec![(key_loc.start_offset(), key_loc.end_offset())];
        let mut outermost_msg_offset: Option<usize> = None;
        let mut current_receiver = call.receiver();

        while let Some(recv) = current_receiver {
            let recv_call = match recv.as_call_node() {
                Some(c) => c,
                None => break,
            };

            if recv_call.name().as_slice() != b"fetch" {
                break;
            }

            let recv_args = match recv_call.arguments() {
                Some(a) => a,
                None => break,
            };
            let recv_arg_list: Vec<_> = recv_args.arguments().iter().collect();
            if recv_arg_list.len() != 2 {
                break;
            }

            if !Self::is_nil_or_empty_hash(&recv_arg_list[1]) {
                break;
            }

            if recv_call.block().is_some() {
                break;
            }

            let k = recv_arg_list[0].location();
            key_ranges.push((k.start_offset(), k.end_offset()));
            let msg_loc = recv_call
                .message_loc()
                .unwrap_or_else(|| recv_call.location());
            outermost_msg_offset = Some(msg_loc.start_offset());
            current_receiver = recv_call.receiver();
        }

        let msg_offset = match outermost_msg_offset {
            Some(o) => o,
            None => return, // no chain found (need at least 2 levels)
        };

        // key_ranges were collected innermost-first; reverse to get outermost-first order
        key_ranges.reverse();

        // Build dig arguments string
        let src_bytes = source.as_bytes();
        let dig_args: Vec<&str> = key_ranges
            .iter()
            .map(|&(start, end)| std::str::from_utf8(&src_bytes[start..end]).unwrap_or("?"))
            .collect();

        let (line, column) = source.offset_to_line_col(msg_offset);

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `dig({})` instead.", dig_args.join(", ")),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashFetchChain, "cops/style/hash_fetch_chain");
}
