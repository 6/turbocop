use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NegativeArrayIndex;

impl Cop for NegativeArrayIndex {
    fn name(&self) -> &'static str {
        "Style/NegativeArrayIndex"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Looking for: arr[arr.length - n] or arr[arr.size - n]
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be `[]` method
        if call.name().as_slice() != b"[]" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        let arg = &arg_list[0];

        // The argument should be a subtraction: something.length - n or something.size - n
        let sub_call = match arg.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if sub_call.name().as_slice() != b"-" {
            return Vec::new();
        }

        let sub_receiver = match sub_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // sub_receiver should be a call to .length or .size on the same receiver
        let length_call = match sub_receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let length_method = length_call.name();
        let length_bytes = length_method.as_slice();
        if length_bytes != b"length" && length_bytes != b"size" && length_bytes != b"count" {
            return Vec::new();
        }

        // The receiver of .length should match the receiver of []
        let length_receiver = match length_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let arr_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("");
        let len_recv_src = std::str::from_utf8(length_receiver.location().as_slice()).unwrap_or("");

        if arr_src != len_recv_src {
            return Vec::new();
        }

        // Get the subtracted value
        let sub_args = match sub_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let sub_arg_list: Vec<_> = sub_args.arguments().iter().collect();
        if sub_arg_list.len() != 1 {
            return Vec::new();
        }
        let n_node = &sub_arg_list[0];
        let n_src = std::str::from_utf8(n_node.location().as_slice()).unwrap_or("");

        let full_src = std::str::from_utf8(node.location().as_slice()).unwrap_or("");
        let loc = arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `{arr_src}[-{n_src}]` instead of `{full_src}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NegativeArrayIndex, "cops/style/negative_array_index");
}
