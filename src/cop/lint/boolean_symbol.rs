use crate::cop::node_type::SYMBOL_NODE;
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let symbol_node = match node.as_symbol_node() {
            Some(n) => n,
            None => return,
        };

        let value = symbol_node.unescaped();
        let boolean_name = if value == b"true" {
            "true"
        } else if value == b"false" {
            "false"
        } else {
            return;
        };

        // Skip :true/:false inside %i[] percent symbol arrays
        // RuboCop: `return if parent&.array_type? && parent.percent_literal?(:symbol)`
        // In Prism, %i[] symbols don't have a `:` prefix in their opening_loc.
        if let Some(open) = symbol_node.opening_loc() {
            // Normal symbol: opening is `:`, %i symbol has no `:` opening
            if !open.as_slice().starts_with(b":") {
                return;
            }
        } else {
            // No opening loc means it's inside a %i[] literal
            return;
        }

        let loc = symbol_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Symbol with a boolean name - you probably meant to use `{boolean_name}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BooleanSymbol, "cops/lint/boolean_symbol");
}
