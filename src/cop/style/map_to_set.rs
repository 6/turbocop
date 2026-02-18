use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MapToSet;

impl Cop for MapToSet {
    fn name(&self) -> &'static str {
        "Style/MapToSet"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Looking for: foo.map { ... }.to_set  or  foo.collect { ... }.to_set
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"to_set" {
            return Vec::new();
        }
        // to_set should have no block of its own
        if call.block().is_some() {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = recv_call.name();
        let method_bytes = method_name.as_slice();
        if method_bytes != b"map" && method_bytes != b"collect" {
            return Vec::new();
        }

        // Must have a block
        if recv_call.block().is_none() {
            return Vec::new();
        }

        let method_str = std::str::from_utf8(method_bytes).unwrap_or("map");
        let msg_loc = recv_call.message_loc().unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Pass a block to `to_set` instead of calling `{method_str}.to_set`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapToSet, "cops/style/map_to_set");
}
