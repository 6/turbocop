use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct BooleanSymbol;

impl Cop for BooleanSymbol {
    fn name(&self) -> &'static str {
        "Lint/BooleanSymbol"
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
        let symbol_node = match node.as_symbol_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let value = symbol_node.unescaped();
        let boolean_name = if value == b"true" {
            "true"
        } else if value == b"false" {
            "false"
        } else {
            return Vec::new();
        };

        let loc = symbol_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Symbol with a boolean name - you probably meant to use `{boolean_name}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BooleanSymbol, "cops/lint/boolean_symbol");
}
