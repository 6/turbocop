use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct LiteralInInterpolation;

impl Cop for LiteralInInterpolation {
    fn name(&self) -> &'static str {
        "Lint/LiteralInInterpolation"
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
        let embedded = match node.as_embedded_statements_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let stmts = match embedded.statements() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body = stmts.body();
        if body.len() != 1 {
            return Vec::new();
        }

        let first = match body.first() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let is_literal = first.as_integer_node().is_some()
            || first.as_float_node().is_some()
            || first.as_string_node().is_some()
            || first.as_symbol_node().is_some()
            || first.as_nil_node().is_some()
            || first.as_true_node().is_some()
            || first.as_false_node().is_some();

        if !is_literal {
            return Vec::new();
        }

        let loc = embedded.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Literal interpolation detected.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LiteralInInterpolation, "cops/lint/literal_in_interpolation");
}
