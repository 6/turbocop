use crate::cop::shared::node_type::{
    BLOCK_ARGUMENT_NODE, CALL_NODE, FORWARDING_ARGUMENTS_NODE, HASH_NODE, KEYWORD_HASH_NODE,
    SPLAT_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FP investigation (2026-03-08): 11 FP (8 activemerchant, 2 world_cup_json, 1 anyway_config).
///
/// Root cause 1: Missing `ForwardingArgumentsNode` check. RuboCop's `IGNORED_ARGUMENT_TYPES`
/// includes `forwarded_args` which maps to Prism's `ForwardingArgumentsNode`. Patterns like
/// `data.dig(...)` (argument forwarding) were being falsely flagged.
///
/// Other forwarding patterns (`dig(*)`, `dig(**)`, `dig(&)`) were already handled by the
/// existing `SplatNode`, `KeywordHashNode`, and `BlockArgumentNode` checks respectively.
///
/// Root cause 2 (2026-03-10): 10 FP from chained `.dig().dig()` calls. RuboCop's
/// `ignore_dig_chain?` skips single-arg dig calls that are part of a chain when
/// `Style/DigChain` is enabled (which it is by default — `Enabled: pending` is truthy).
/// Fixed by checking if the receiver is a dig call OR if `.dig(` follows in the source.
pub struct SingleArgumentDig;

impl Cop for SingleArgumentDig {
    fn name(&self) -> &'static str {
        "Style/SingleArgumentDig"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_ARGUMENT_NODE,
            CALL_NODE,
            FORWARDING_ARGUMENTS_NODE,
            HASH_NODE,
            KEYWORD_HASH_NODE,
            SPLAT_NODE,
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

        // Must be a call to .dig
        if call.name().as_slice() != b"dig" {
            return;
        }

        // Must have a receiver (not safe navigation)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Skip safe navigation calls (foo&.dig)
        if let Some(op_loc) = call.call_operator_loc() {
            if op_loc.as_slice() == b"&." {
                return;
            }
        }

        // Must have exactly one argument
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // Skip block_pass, splat, hash, and forwarding arguments
        let arg = &arg_list[0];
        if arg.as_block_argument_node().is_some()
            || arg.as_splat_node().is_some()
            || arg.as_keyword_hash_node().is_some()
            || arg.as_hash_node().is_some()
            || arg.as_forwarding_arguments_node().is_some()
        {
            return;
        }

        // Skip chained dig calls (RuboCop's ignore_dig_chain?).
        // Check if receiver is a dig call (this is the outer dig in a.dig(x).dig(y))
        if let Some(recv_call) = receiver.as_call_node() {
            if recv_call.name().as_slice() == b"dig" {
                return;
            }
        }
        // Check if this dig call is the receiver of another dig call by looking
        // at the source bytes immediately after this node for ".dig("
        let end = node.location().end_offset();
        let remaining = &source.as_bytes()[end..];
        if remaining.starts_with(b".dig(") {
            return;
        }

        let recv_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("hash");
        let arg_src = std::str::from_utf8(arg.location().as_slice()).unwrap_or(":key");
        let original = std::str::from_utf8(node.location().as_slice()).unwrap_or("");
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{}[{}]` instead of `{}`.", recv_src, arg_src, original),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SingleArgumentDig, "cops/style/single_argument_dig");
}
