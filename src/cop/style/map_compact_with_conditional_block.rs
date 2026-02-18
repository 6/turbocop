use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MapCompactWithConditionalBlock;

impl Cop for MapCompactWithConditionalBlock {
    fn name(&self) -> &'static str {
        "Style/MapCompactWithConditionalBlock"
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

        // Check for .compact call
        if call.name().as_slice() != b"compact" {
            return Vec::new();
        }

        // No arguments for compact
        if call.arguments().is_some() {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Receiver should be a .map call with a block
        let map_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if map_call.name().as_slice() != b"map" {
            return Vec::new();
        }

        // map call must have a block
        let block = match map_call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Check if block body is a conditional (if/unless) that returns nil in one branch
        let body = match block_node.body() {
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

        let expr = &body_nodes[0];

        // Check if it's an if/unless with nil return
        let is_conditional_with_nil = if let Some(if_node) = expr.as_if_node() {
            // The else branch is implicitly nil if absent
            if_node.subsequent().is_none()
        } else {
            false
        };

        if is_conditional_with_nil {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use `filter_map` instead of `map { ... }.compact`.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapCompactWithConditionalBlock, "cops/style/map_compact_with_conditional_block");
}
