use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::IF_NODE;

pub struct MultilineTernaryOperator;

impl Cop for MultilineTernaryOperator {
    fn name(&self) -> &'static str {
        "Style/MultilineTernaryOperator"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE]
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

        // Must be a ternary (no if_keyword_loc)
        if if_node.if_keyword_loc().is_some() {
            return Vec::new();
        }

        // Must be multiline
        let loc = if_node.location();
        let (start_line, _) = source.offset_to_line_col(loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));

        if start_line == end_line {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid multi-line ternary operators, use `if` or `unless` instead.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineTernaryOperator, "cops/style/multiline_ternary_operator");
}
