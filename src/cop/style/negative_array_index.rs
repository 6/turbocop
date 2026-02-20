use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct NegativeArrayIndex;

impl Cop for NegativeArrayIndex {
    fn name(&self) -> &'static str {
        "Style/NegativeArrayIndex"
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
        // Looking for: arr[arr.length - n] or arr[arr.size - n]
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be `[]` method
        if call.name().as_slice() != b"[]" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        let arg = &arg_list[0];

        // The argument should be a subtraction: something.length - n or something.size - n
        let sub_call = match arg.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if sub_call.name().as_slice() != b"-" {
            return;
        }

        let sub_receiver = match sub_call.receiver() {
            Some(r) => r,
            None => return,
        };

        // sub_receiver should be a call to .length or .size on the same receiver
        let length_call = match sub_receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let length_method = length_call.name();
        let length_bytes = length_method.as_slice();
        if length_bytes != b"length" && length_bytes != b"size" && length_bytes != b"count" {
            return;
        }

        // The receiver of .length should match the receiver of []
        let length_receiver = match length_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let arr_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("");
        let len_recv_src = std::str::from_utf8(length_receiver.location().as_slice()).unwrap_or("");

        if arr_src != len_recv_src {
            return;
        }

        // Get the subtracted value
        let sub_args = match sub_call.arguments() {
            Some(a) => a,
            None => return,
        };
        let sub_arg_list: Vec<_> = sub_args.arguments().iter().collect();
        if sub_arg_list.len() != 1 {
            return;
        }
        let n_node = &sub_arg_list[0];
        let n_src = std::str::from_utf8(n_node.location().as_slice()).unwrap_or("");

        let full_src = std::str::from_utf8(node.location().as_slice()).unwrap_or("");
        let loc = arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{arr_src}[-{n_src}]` instead of `{full_src}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NegativeArrayIndex, "cops/style/negative_array_index");
}
