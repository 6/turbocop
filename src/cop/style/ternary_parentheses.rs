use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TernaryParentheses;

impl Cop for TernaryParentheses {
    fn name(&self) -> &'static str {
        "Style/TernaryParentheses"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Ternary has no if_keyword_loc
        if if_node.if_keyword_loc().is_some() {
            return Vec::new();
        }

        // Check if condition is wrapped in parentheses
        if let Some(paren) = if_node.predicate().as_parentheses_node() {
            let open_loc = paren.opening_loc();
            let (line, column) = source.offset_to_line_col(open_loc.start_offset());
            return vec![self.diagnostic(source, line, column, "Ternary conditions should not be wrapped in parentheses.".to_string())];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(TernaryParentheses, "cops/style/ternary_parentheses");
}
