use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantFilterChain;

const FILTER_METHODS: &[&[u8]] = &[b"select", b"filter", b"find_all"];

impl Cop for RedundantFilterChain {
    fn name(&self) -> &'static str {
        "Style/RedundantFilterChain"
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

        let method_bytes = call.name().as_slice();

        // Must be any?, empty?, none?, or one?
        let replacement = match method_bytes {
            b"any?" => "any?",
            b"empty?" => "none?",
            b"none?" => "none?",
            b"one?" => "one?",
            _ => return Vec::new(),
        };

        // Must have no arguments or block
        if call.arguments().is_some() || call.block().is_some() {
            return Vec::new();
        }

        // Receiver must be a filter method with a block
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let recv_method = recv_call.name();
        let recv_bytes = recv_method.as_slice();

        if !FILTER_METHODS.iter().any(|&m| m == recv_bytes) {
            return Vec::new();
        }

        // The filter method must have a block (or block pass)
        if recv_call.block().is_none() {
            return Vec::new();
        }

        let filter_str = std::str::from_utf8(recv_bytes).unwrap_or("select");
        let predicate_str = std::str::from_utf8(method_bytes).unwrap_or("any?");

        let msg_loc = recv_call.message_loc().unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `{replacement}` instead of `{filter_str}.{predicate_str}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantFilterChain, "cops/style/redundant_filter_chain");
}
