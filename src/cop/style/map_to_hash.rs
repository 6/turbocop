use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct MapToHash;

impl Cop for MapToHash {
    fn name(&self) -> &'static str {
        "Style/MapToHash"
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
        // Looking for: foo.map { ... }.to_h  or  foo.collect { ... }.to_h
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // The outer call must be `to_h` with no arguments and no block
        if call.name().as_slice() != b"to_h" {
            return;
        }
        if call.arguments().is_some() || call.block().is_some() {
            return;
        }

        // The receiver must be a call to `map` or `collect` with a block
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

        // Must have a block (not just arguments)
        if recv_call.block().is_none() {
            return;
        }

        let method_str = std::str::from_utf8(method_bytes).unwrap_or("map");
        let msg_loc = recv_call.message_loc().unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Pass a block to `to_h` instead of calling `{method_str}.to_h`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapToHash, "cops/style/map_to_hash");
}
