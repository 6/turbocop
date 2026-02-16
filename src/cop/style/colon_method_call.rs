use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ColonMethodCall;

impl Cop for ColonMethodCall {
    fn name(&self) -> &'static str {
        "Style/ColonMethodCall"
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

        // Must have a receiver
        if call_node.receiver().is_none() {
            return Vec::new();
        }

        // Must use :: as the call operator
        let call_op_loc = match call_node.call_operator_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        if call_op_loc.as_slice() != b"::" {
            return Vec::new();
        }

        // The method name must start with a lowercase letter or underscore
        // (i.e., it's a regular method, not a constant access)
        let method_name = call_node.name();
        let name_bytes = method_name.as_slice();
        if name_bytes.is_empty() {
            return Vec::new();
        }

        let first = name_bytes[0];
        // Skip if it starts with uppercase (constant access like Foo::Bar)
        if first.is_ascii_uppercase() {
            return Vec::new();
        }

        // Skip Java-style names (e.g., Java::int, Java::com)
        // These are common in JRuby
        if let Some(receiver) = call_node.receiver() {
            if let Some(cr) = receiver.as_constant_read_node() {
                if cr.name().as_slice() == b"Java" {
                    return Vec::new();
                }
            }
            // Also handle qualified constants (e.g., SomeModule::Java::method)
            if let Some(cp) = receiver.as_constant_path_node() {
                let cp_src = cp.location().as_slice();
                if cp_src.ends_with(b"Java") {
                    return Vec::new();
                }
            }
        }

        let (line, column) = source.offset_to_line_col(call_op_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not use `::` for method calls.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ColonMethodCall, "cops/style/colon_method_call");
}
