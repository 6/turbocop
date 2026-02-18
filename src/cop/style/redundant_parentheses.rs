use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantParentheses;

impl Cop for RedundantParentheses {
    fn name(&self) -> &'static str {
        "Style/RedundantParentheses"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let parens = match node.as_parentheses_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let body = match parens.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Check if the parenthesized expression is a literal, variable, or keyword
        let msg = if is_literal(&body) {
            "a literal"
        } else if is_variable(&body) {
            "a variable"
        } else if is_keyword(&body) {
            "a keyword"
        } else if is_constant(&body) {
            "a constant"
        } else {
            return Vec::new();
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Don't use parentheses around {}.", msg),
        )]
    }
}

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_array_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
}

fn is_variable(node: &ruby_prism::Node<'_>) -> bool {
    node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
}

fn is_keyword(node: &ruby_prism::Node<'_>) -> bool {
    node.as_self_node().is_some()
        || node.as_source_file_node().is_some()
        || node.as_source_line_node().is_some()
        || node.as_source_encoding_node().is_some()
}

fn is_constant(node: &ruby_prism::Node<'_>) -> bool {
    node.as_constant_read_node().is_some()
        || node.as_constant_path_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantParentheses, "cops/style/redundant_parentheses");
}
