use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Inquiry;

impl Cop for Inquiry {
    fn name(&self) -> &'static str {
        "Rails/Inquiry"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        if call.name().as_slice() != b"inquiry" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // RuboCop only flags inquiry when the receiver is a string literal or array literal.
        // Method call results (e.g., `ROLES.key(flag)&.inquiry`) are not flagged.
        let is_string_or_array = receiver.as_string_node().is_some()
            || receiver.as_interpolated_string_node().is_some()
            || receiver.as_array_node().is_some();
        if !is_string_or_array {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid `String#inquiry`. Use direct comparison or predicate methods.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Inquiry, "cops/rails/inquiry");
}
