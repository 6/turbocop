use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TrailingCommaInAttributeDeclaration;

const ATTR_METHODS: &[&[u8]] = &[b"attr_reader", b"attr_writer", b"attr_accessor", b"attr"];

impl Cop for TrailingCommaInAttributeDeclaration {
    fn name(&self) -> &'static str {
        "Lint/TrailingCommaInAttributeDeclaration"
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

        // Must be a bare call (no receiver) to an attr method
        if call.receiver().is_some() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if !ATTR_METHODS.iter().any(|m| *m == method_name) {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Check if the last argument is a DefNode (method definition).
        // This happens when there's a trailing comma in the attribute declaration:
        // `attr_reader :foo,` followed by `def bar; end` causes the `def` to be
        // parsed as an argument to `attr_reader`.
        let last_arg = &arg_list[arg_list.len() - 1];
        if last_arg.as_def_node().is_some() {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Avoid leaving a trailing comma in attribute declarations.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TrailingCommaInAttributeDeclaration,
        "cops/lint/trailing_comma_in_attribute_declaration"
    );
}
