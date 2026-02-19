use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, IF_NODE, STATEMENTS_NODE};

pub struct MapCompactWithConditionalBlock;

impl Cop for MapCompactWithConditionalBlock {
    fn name(&self) -> &'static str {
        "Style/MapCompactWithConditionalBlock"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, IF_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Check for .compact call
        if call.name().as_slice() != b"compact" {
            return;
        }

        // No arguments for compact
        if call.arguments().is_some() {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver should be a .map call with a block
        let map_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if map_call.name().as_slice() != b"map" {
            return;
        }

        // map call must have a block
        let block = match map_call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Check if block body is a conditional (if/unless) that returns nil in one branch
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return;
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
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use `filter_map` instead of `map { ... }.compact`.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapCompactWithConditionalBlock, "cops/style/map_compact_with_conditional_block");
}
