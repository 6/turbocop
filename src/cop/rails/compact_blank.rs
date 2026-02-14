use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct CompactBlank;

impl Cop for CompactBlank {
    fn name(&self) -> &'static str {
        "Rails/CompactBlank"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        let method_name = call.name().as_slice();

        // Must be .reject or .select with a block
        let (expected_predicate, msg_fragment) = if method_name == b"reject" {
            (b"blank?" as &[u8], "reject { |e| e.blank? }")
        } else if method_name == b"select" {
            (b"present?" as &[u8], "select { |e| e.present? }")
        } else {
            return Vec::new();
        };

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Block should have parameters (|x|)
        if block_node.parameters().is_none() {
            return Vec::new();
        }

        // Block body should be a single call to .blank? or .present?
        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<ruby_prism::Node<'_>> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return Vec::new();
        }

        let body_call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if body_call.name().as_slice() != expected_predicate {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `compact_blank` instead of `{msg_fragment}`."),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CompactBlank, "cops/rails/compact_blank");
}
