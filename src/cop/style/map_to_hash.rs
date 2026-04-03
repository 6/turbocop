use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// Current reruns showed both over- and under-reporting:
/// - FP examples from `opal` and `inspec` were arbitrary block-pass calls like
///   `map(&block).to_h`. RuboCop only matches literal blocks and symbol block-pass
///   (`&:sym`), not generic `&blk`.
/// - Fixture gaps also showed the upstream cop preserves the outer call operator in
///   the message (`map&.to_h` for `&.to_h`) and only runs on Ruby >= 2.6.
///
/// Fix: match RuboCop more closely by allowing only block literals or `&:sym`,
/// preserving `&.` in the offense message, and adding the Ruby 2.6 version gate.
/// Acceptance gate after fix: `scripts/check-cop.py Style/MapToHash --verbose --rerun`
/// reported Expected=395, Actual=395, Excess=0, Missing=0.
pub struct MapToHash;

fn target_ruby_version(config: &CopConfig) -> f64 {
    config
        .options
        .get("TargetRubyVersion")
        .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
        .unwrap_or(2.7)
}

fn is_map_or_collect(call: &ruby_prism::CallNode<'_>) -> bool {
    matches!(call.name().as_slice(), b"map" | b"collect")
}

fn has_symbol_block_pass(call: &ruby_prism::CallNode<'_>) -> bool {
    call.block()
        .and_then(|block| block.as_block_argument_node())
        .and_then(|block_arg| block_arg.expression())
        .is_some_and(|expr| expr.as_symbol_node().is_some())
}

fn has_block_literal(call: &ruby_prism::CallNode<'_>) -> bool {
    call.block()
        .is_some_and(|block| block.as_block_argument_node().is_none())
}

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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // RuboCop: minimum_target_ruby_version 2.6
        if target_ruby_version(config) < 2.6 {
            return;
        }

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

        // The receiver must be a call to `map` or `collect` with either:
        // - a literal block (`{ ... }` / `do ... end`)
        // - a symbol block-pass (`&:method`)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if !is_map_or_collect(&recv_call) {
            return;
        }

        if !has_block_literal(&recv_call) && !has_symbol_block_pass(&recv_call) {
            return;
        }

        let method_bytes = recv_call.name().as_slice();
        let method_str = std::str::from_utf8(method_bytes).unwrap_or("map");
        let dot = call
            .call_operator_loc()
            .and_then(|loc| std::str::from_utf8(loc.as_slice()).ok())
            .unwrap_or(".");
        let msg_loc = recv_call
            .message_loc()
            .unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Pass a block to `to_h` instead of calling `{method_str}{dot}to_h`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapToHash, "cops/style/map_to_hash");
}
