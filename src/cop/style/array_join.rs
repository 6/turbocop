use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArrayJoin;

impl Cop for ArrayJoin {
    fn name(&self) -> &'static str {
        "Style/ArrayJoin"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be the `*` method
        if call_node.name().as_slice() != b"*" {
            return Vec::new();
        }

        // The receiver must be an array literal
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if receiver.as_array_node().is_none() {
            return Vec::new();
        }

        // The argument must be a string literal
        let args = match call_node.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return Vec::new();
        }

        if arg_list[0].as_string_node().is_none() && arg_list[0].as_interpolated_string_node().is_none() {
            return Vec::new();
        }

        let msg_loc = call_node.message_loc().unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Favor `Array#join` over `Array#*`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArrayJoin, "cops/style/array_join");
}
