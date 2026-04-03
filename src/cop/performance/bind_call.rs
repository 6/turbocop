use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Detects `bind(obj).call(args)` that can be replaced with `bind_call(obj, args)`.
///
/// Fixed: bare `bind(object)` without an explicit receiver was rejected (receiver().is_none()
/// guard), causing FN on patterns like `bind(object).call(*args, &block)`. RuboCop's NodePattern
/// uses `_` which matches nil receivers. Also fixed: block pass arguments (`&block`) on the
/// `.call()` side were not included in the suggested replacement message because Prism puts them
/// in `call.block()` rather than `call.arguments()`.
pub struct BindCall;

impl Cop for BindCall {
    fn name(&self) -> &'static str {
        "Performance/BindCall"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        // Detect: receiver.bind(obj).call(args...)
        // Pattern: (send (send _ :bind $arg) :call $...)
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if outer_call.name().as_slice() != b"call" {
            return;
        }

        let bind_node = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let bind_call = match bind_node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if bind_call.name().as_slice() != b"bind" {
            return;
        }

        // Extract bind argument source
        let bind_args = match bind_call.arguments() {
            Some(a) => a,
            None => return,
        };
        let bind_arg_list: Vec<_> = bind_args.arguments().iter().collect();
        if bind_arg_list.len() != 1 {
            return;
        }
        let bytes = source.as_bytes();
        let bind_arg_src = std::str::from_utf8(
            &bytes[bind_arg_list[0].location().start_offset()
                ..bind_arg_list[0].location().end_offset()],
        )
        .unwrap_or("obj");

        // Extract call arguments source (positional args + block pass)
        let mut call_arg_parts: Vec<String> = Vec::new();
        if let Some(call_args) = outer_call.arguments() {
            for a in call_args.arguments().iter() {
                let s = std::str::from_utf8(
                    &bytes[a.location().start_offset()..a.location().end_offset()],
                )
                .unwrap_or("?");
                call_arg_parts.push(s.to_string());
            }
        }
        // Include block pass argument (&block) if present
        if let Some(block) = outer_call.block() {
            if block.as_block_argument_node().is_some() {
                let s = std::str::from_utf8(
                    &bytes[block.location().start_offset()..block.location().end_offset()],
                )
                .unwrap_or("&block");
                call_arg_parts.push(s.to_string());
            }
        }
        let call_args_src = call_arg_parts.join(", ");

        let comma = if call_args_src.is_empty() { "" } else { ", " };
        let msg = format!(
            "Use `bind_call({bind_arg_src}{comma}{call_args_src})` instead of `bind({bind_arg_src}).call({call_args_src})`."
        );

        // Report at the .bind selector position (matching RuboCop's correction_range)
        let bind_msg_loc = match bind_call.message_loc() {
            Some(loc) => loc,
            None => return,
        };
        let (line, column) = source.offset_to_line_col(bind_msg_loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BindCall, "cops/performance/bind_call");
}
