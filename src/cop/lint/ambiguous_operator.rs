use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, SPLAT_NODE};

/// Checks for ambiguous operators in the first argument of a method invocation
/// without parentheses. For example, `do_something *some_array` where `*` could
/// be interpreted as either a splat or multiplication.
pub struct AmbiguousOperator;

impl Cop for AmbiguousOperator {
    fn name(&self) -> &'static str {
        "Lint/AmbiguousOperator"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, SPLAT_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Only check method calls without parentheses
        if call.opening_loc().is_some() {
            return;
        }

        // Must be a regular method call (not an operator)
        let name = call.name().as_slice();
        if name.iter().all(|b| !b.is_ascii_alphabetic() && *b != b'_') {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let first_arg = &arg_list[0];

        // Check for splat (*x)
        if let Some(splat) = first_arg.as_splat_node() {
            let loc = splat.location();
            let start = loc.start_offset();
            // Check there's no space after the *
            if start > 0 {
                let src = &source.as_bytes()[start..];
                if src.starts_with(b"*") && src.len() > 1 && src[1] != b' ' {
                    let (line, column) = source.offset_to_line_col(start);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Ambiguous splat operator. Parenthesize the method arguments if it's surely a splat operator, or add a whitespace to the right of the `*` if it should be a multiplication.".to_string(),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AmbiguousOperator, "cops/lint/ambiguous_operator");
}
