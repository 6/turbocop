use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

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

        // bind must have a receiver (not bare `bind(...)`)
        if bind_call.receiver().is_none() {
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
            &bytes[bind_arg_list[0].location().start_offset()..bind_arg_list[0].location().end_offset()]
        ).unwrap_or("obj");

        // Extract call arguments source
        let call_args_src = if let Some(call_args) = outer_call.arguments() {
            let args: Vec<_> = call_args.arguments().iter().map(|a| {
                std::str::from_utf8(&bytes[a.location().start_offset()..a.location().end_offset()])
                    .unwrap_or("?")
            }).collect();
            args.join(", ")
        } else {
            String::new()
        };

        let comma = if call_args_src.is_empty() { "" } else { ", " };
        let msg = format!(
            "Use `bind_call({bind_arg_src}{comma}{call_args_src})` instead of `bind({bind_arg_src}).call({call_args_src})`."
        );

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BindCall, "cops/performance/bind_call");
}
