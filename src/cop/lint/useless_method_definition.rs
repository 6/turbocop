use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UselessMethodDefinition;

impl Cop for UselessMethodDefinition {
    fn name(&self) -> &'static str {
        "Lint/UselessMethodDefinition"
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
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return Vec::new();
        }

        // Check if the single statement is a `super` call
        let first = &body_nodes[0];

        // ForwardingSuperNode is `super` with implicit forwarding
        if first.as_forwarding_super_node().is_some() {
            let loc = def_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Useless method definition detected. The method just delegates to `super`."
                    .to_string(),
            )];
        }

        // SuperNode is explicit `super(args)`
        if first.as_super_node().is_some() {
            let loc = def_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Useless method definition detected. The method just delegates to `super`."
                    .to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessMethodDefinition, "cops/lint/useless_method_definition");
}
