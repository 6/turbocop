use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/MapToSet: detects `map { ... }.to_set` and suggests `to_set { ... }`.
///
/// FP fix: RuboCop's pattern `(block_pass sym)` only matches `&:symbol`, not `&variable`
/// or `&method(:foo)`. We must check that block_pass arguments contain a symbol node
/// before flagging.
pub struct MapToSet;

impl Cop for MapToSet {
    fn name(&self) -> &'static str {
        "Style/MapToSet"
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
        // Looking for: foo.map { ... }.to_set  or  foo.collect { ... }.to_set
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"to_set" {
            return;
        }
        // to_set should have no block of its own
        if call.block().is_some() {
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

        let method_name = recv_call.name();
        let method_bytes = method_name.as_slice();
        if method_bytes != b"map" && method_bytes != b"collect" {
            return;
        }

        // Must have a block
        let block = match recv_call.block() {
            Some(b) => b,
            None => return,
        };

        // If the block is a block_pass (&expr), only flag when expr is a symbol (&:sym).
        // RuboCop's pattern is `(block_pass sym)` which only matches symbol literals,
        // not variables (&var) or method calls (&method(:foo)).
        if let Some(block_arg) = block.as_block_argument_node() {
            let is_symbol = block_arg
                .expression()
                .is_some_and(|expr| expr.as_symbol_node().is_some());
            if !is_symbol {
                return;
            }
        }

        let method_str = std::str::from_utf8(method_bytes).unwrap_or("map");
        let msg_loc = recv_call
            .message_loc()
            .unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Pass a block to `to_set` instead of calling `{method_str}.to_set`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapToSet, "cops/style/map_to_set");
}
