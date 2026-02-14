use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RaiseException;

fn is_exception_reference(node: &ruby_prism::Node<'_>) -> bool {
    // Direct constant: Exception
    if let Some(c) = node.as_constant_read_node() {
        return c.name().as_slice() == b"Exception";
    }
    // Exception.new(...)
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"new" {
            if let Some(recv) = call.receiver() {
                if let Some(c) = recv.as_constant_read_node() {
                    return c.name().as_slice() == b"Exception";
                }
            }
        }
    }
    false
}

impl Cop for RaiseException {
    fn name(&self) -> &'static str {
        "Lint/RaiseException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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

        // Must be a receiverless raise or fail
        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if method_name != b"raise" && method_name != b"fail" {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        let first_arg = match args.first() {
            Some(a) => a,
            None => return Vec::new(),
        };

        if !is_exception_reference(&first_arg) {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use a subclass of `Exception` instead of raising `Exception` directly.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RaiseException, "cops/lint/raise_exception");
}
