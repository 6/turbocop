use crate::cop::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE, STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ImplicitRuntimeError;

impl Cop for ImplicitRuntimeError {
    fn name(&self) -> &'static str {
        "Style/ImplicitRuntimeError"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTERPOLATED_STRING_NODE,
            STRING_NODE,
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

        let method_bytes = call.name().as_slice();

        // Only check raise and fail
        if method_bytes != b"raise" && method_bytes != b"fail" {
            return;
        }

        // Must have no explicit receiver (or Kernel/::Kernel receiver)
        if let Some(recv) = call.receiver() {
            let is_kernel = recv
                .as_constant_read_node()
                .is_some_and(|c| c.name().as_slice() == b"Kernel")
                || recv.as_constant_path_node().is_some_and(|cp| {
                    cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"Kernel")
                });
            if !is_kernel {
                return;
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return, // raise/fail with no args is OK
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // If the first argument is a string, it's an offense
        let first_arg = &arg_list[0];
        let is_string = first_arg.as_string_node().is_some()
            || first_arg.as_interpolated_string_node().is_some();

        if is_string && arg_list.len() == 1 {
            let method_str = std::str::from_utf8(method_bytes).unwrap_or("raise");
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Use `{}` with an explicit exception class and message, rather than just a message.",
                    method_str
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ImplicitRuntimeError, "cops/style/implicit_runtime_error");
}
