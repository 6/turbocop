use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE, INTERPOLATED_X_STRING_NODE, STRING_NODE, X_STRING_NODE};

pub struct EvalWithLocation;

const EVAL_METHODS: &[&[u8]] = &[b"eval", b"class_eval", b"module_eval", b"instance_eval"];

impl EvalWithLocation {
    fn is_eval_method(name: &[u8]) -> bool {
        EVAL_METHODS.contains(&name)
    }

    fn requires_binding(name: &[u8]) -> bool {
        name == b"eval"
    }

    fn is_string_arg(node: &ruby_prism::Node<'_>) -> bool {
        node.as_string_node().is_some()
            || node.as_interpolated_string_node().is_some()
            || node.as_x_string_node().is_some()
            || node.as_interpolated_x_string_node().is_some()
    }

    fn is_heredoc_arg(node: &ruby_prism::Node<'_>) -> bool {
        // Heredoc nodes in prism are string nodes with heredoc opening
        if let Some(s) = node.as_string_node() {
            if let Some(opening) = s.opening_loc() {
                let opening_bytes = opening.as_slice();
                return opening_bytes.starts_with(b"<<");
            }
        }
        if let Some(s) = node.as_interpolated_string_node() {
            if let Some(opening) = s.opening_loc() {
                let opening_bytes = opening.as_slice();
                return opening_bytes.starts_with(b"<<");
            }
        }
        false
    }
}

impl Cop for EvalWithLocation {
    fn name(&self) -> &'static str {
        "Style/EvalWithLocation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE, INTERPOLATED_X_STRING_NODE, STRING_NODE, X_STRING_NODE]
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

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        if !Self::is_eval_method(method_bytes) {
            return Vec::new();
        }

        // Check if it has a block - if so, skip (block form doesn't need file/line)
        if call.block().is_some() {
            return Vec::new();
        }

        let receiver = call.receiver();

        // For `eval`, only allow no receiver, Kernel, or ::Kernel
        if method_bytes == b"eval" {
            if let Some(ref recv) = receiver {
                let is_kernel = recv.as_constant_read_node()
                    .map_or(false, |c| c.name().as_slice() == b"Kernel");
                let is_scoped_kernel = recv.as_constant_path_node().map_or(false, |cp| {
                    cp.parent().is_none()
                        && cp.name().map_or(false, |n| n.as_slice() == b"Kernel")
                });
                if !is_kernel && !is_scoped_kernel {
                    return Vec::new();
                }
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => {
                // No arguments at all - register offense
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let needs_binding = Self::requires_binding(method_bytes);
                let method_str = std::str::from_utf8(method_bytes).unwrap_or("eval");
                let msg = if needs_binding {
                    format!("Pass a binding, `__FILE__`, and `__LINE__` to `{}`.", method_str)
                } else {
                    format!("Pass `__FILE__` and `__LINE__` to `{}`.", method_str)
                };
                return vec![self.diagnostic(source, line, column, msg)];
            }
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();

        if arg_list.is_empty() {
            return Vec::new();
        }

        // First arg must be a string-like expression (code to eval)
        let first_arg = &arg_list[0];

        // If first arg is not a string/heredoc, it might be a variable - skip
        if !Self::is_string_arg(first_arg) {
            return Vec::new();
        }

        let needs_binding = Self::requires_binding(method_bytes);
        let method_str = std::str::from_utf8(method_bytes).unwrap_or("eval");

        // For eval: need (code, binding, __FILE__, __LINE__)
        // For class_eval/module_eval/instance_eval: need (code, __FILE__, __LINE__)
        let expected_count = if needs_binding { 4 } else { 3 };

        if arg_list.len() < expected_count {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let msg = if needs_binding {
                format!("Pass a binding, `__FILE__`, and `__LINE__` to `{}`.", method_str)
            } else {
                format!("Pass `__FILE__` and `__LINE__` to `{}`.", method_str)
            };
            return vec![self.diagnostic(source, line, column, msg)];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EvalWithLocation, "cops/style/eval_with_location");
}
