use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ConcatArrayLiterals;

impl Cop for ConcatArrayLiterals {
    fn name(&self) -> &'static str {
        "Style/ConcatArrayLiterals"
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

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name != "concat" {
            return Vec::new();
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return Vec::new();
        }

        // Must have arguments
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // All arguments must be array literals
        let all_arrays = arg_list.iter().all(|arg| arg.as_array_node().is_some());
        if !all_arrays {
            return Vec::new();
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        // Build a message about the elements
        let msg = "Use `push` with elements as arguments instead of `concat` with array brackets.";

        vec![self.diagnostic(source, line, column, msg.to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConcatArrayLiterals, "cops/style/concat_array_literals");
}
