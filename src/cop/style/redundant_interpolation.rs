use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantInterpolation;

impl Cop for RedundantInterpolation {
    fn name(&self) -> &'static str {
        "Style/RedundantInterpolation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let interp_node = match node.as_interpolated_string_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Must have exactly one part that is an embedded statements node
        let parts: Vec<_> = interp_node.parts().into_iter().collect();
        if parts.len() != 1 {
            return Vec::new();
        }

        let embedded = match parts[0].as_embedded_statements_node() {
            Some(e) => e,
            None => return Vec::new(),
        };

        // Must have exactly one statement inside #{...}
        let statements = match embedded.statements() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body: Vec<_> = statements.body().into_iter().collect();
        if body.len() != 1 {
            return Vec::new();
        }

        // Skip if the inner expression is a string literal (that would be double-interpolation)
        let inner = &body[0];
        if inner.as_string_node().is_some() || inner.as_interpolated_string_node().is_some() {
            return Vec::new();
        }

        let loc = interp_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer `to_s` over string interpolation.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantInterpolation, "cops/style/redundant_interpolation");
}
