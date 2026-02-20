use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct NestedParenthesizedCalls;

impl Cop for NestedParenthesizedCalls {
    fn name(&self) -> &'static str {
        "Style/NestedParenthesizedCalls"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_methods = config.get_string_array("AllowedMethods");

        // Looking for outer_method(inner_method arg) where inner_method has no parens
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Outer call must have actual parentheses (not [] brackets)
        let opening = match outer_call.opening_loc() {
            Some(loc) => loc,
            None => return,
        };
        // Skip [] and []= calls — brackets are not parentheses
        if opening.as_slice() == b"[" {
            return;
        }

        let args = match outer_call.arguments() {
            Some(a) => a,
            None => return,
        };


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

            // Skip setter methods (ending with =)
            if inner_bytes.last() == Some(&b'=') && inner_bytes.len() > 1 && inner_bytes[inner_bytes.len() - 2] != b'!' {
                continue;
            }

            // Check AllowedMethods - only allowed when outer has 1 arg and inner has 1 arg
            if let Some(ref allowed) = allowed_methods {
                let name_str = std::str::from_utf8(inner_bytes).unwrap_or("");
                let outer_arg_count = args.arguments().iter().count();
                let inner_arg_count = inner_call.arguments().map(|a| a.arguments().iter().count()).unwrap_or(0);
                if outer_arg_count == 1 && inner_arg_count == 1 && allowed.iter().any(|m| m == name_str) {
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

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedParenthesizedCalls, "cops/style/nested_parenthesized_calls");
}
