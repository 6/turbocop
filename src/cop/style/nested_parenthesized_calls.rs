use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NestedParenthesizedCalls;

impl Cop for NestedParenthesizedCalls {
    fn name(&self) -> &'static str {
        "Style/NestedParenthesizedCalls"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allowed_methods = config.get_string_array("AllowedMethods");

        // Looking for outer_method(inner_method arg) where inner_method has no parens
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Outer call must have parentheses
        if outer_call.opening_loc().is_none() {
            return Vec::new();
        }

        let args = match outer_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        for arg in args.arguments().iter() {
            let inner_call = match arg.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            // Inner call must NOT have parentheses
            if inner_call.opening_loc().is_some() {
                continue;
            }

            // Inner call must have arguments (otherwise it's just a method call)
            if inner_call.arguments().is_none() {
                continue;
            }

            // Must have a method name (not an operator)
            let inner_name = inner_call.name();
            let inner_bytes = inner_name.as_slice();

            // Skip operators
            if inner_bytes.iter().all(|b| !b.is_ascii_alphanumeric() && *b != b'_' && *b != b'?' && *b != b'!') {
                continue;
            }

            // Check AllowedMethods
            if let Some(ref allowed) = allowed_methods {
                let name_str = std::str::from_utf8(inner_bytes).unwrap_or("");
                if allowed.iter().any(|m| m == name_str) {
                    continue;
                }
            }

            let inner_src = std::str::from_utf8(inner_call.location().as_slice()).unwrap_or("");
            let loc = inner_call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Add parentheses to nested method call `{inner_src}`."),
            ));
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedParenthesizedCalls, "cops/style/nested_parenthesized_calls");
}
