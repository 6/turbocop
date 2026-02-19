use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{DEFINED_NODE, INTERPOLATED_STRING_NODE, INTERPOLATED_SYMBOL_NODE, STRING_NODE, SYMBOL_NODE};

/// Checks for calls to `defined?` with strings or symbols as the argument.
/// Such calls will always return `'expression'`.
pub struct UselessDefined;

impl Cop for UselessDefined {
    fn name(&self) -> &'static str {
        "Lint/UselessDefined"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEFINED_NODE, INTERPOLATED_STRING_NODE, INTERPOLATED_SYMBOL_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let defined_node = match node.as_defined_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let value = defined_node.value();

        let type_name = if value.as_string_node().is_some() || value.as_interpolated_string_node().is_some() {
            "string"
        } else if value.as_symbol_node().is_some() || value.as_interpolated_symbol_node().is_some() {
            "symbol"
        } else {
            return Vec::new();
        };

        let loc = defined_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Calling `defined?` with a {} argument will always return a truthy value.",
                type_name
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessDefined, "cops/lint/useless_defined");
}
