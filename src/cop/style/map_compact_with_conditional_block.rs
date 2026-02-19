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
            // Extract the block parameter name
            let block_param_name = get_block_param_name(&block_node);

            // Only flag when the truthy branch returns the block parameter itself,
            // making it equivalent to select/reject. If the return value is a
            // different expression (e.g., Regexp.last_match(1)), skip it because
            // it can't be replaced with select/reject.
            if let Some(param_name) = block_param_name {
                if let Some(if_node) = expr.as_if_node() {
                    if !truthy_branch_returns_param(source, &if_node, &param_name) {
                        return;
                    }
                }
            }

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

/// Extract the first block parameter name (e.g., `|x|` -> "x").
fn get_block_param_name(block_node: &ruby_prism::BlockNode<'_>) -> Option<Vec<u8>> {
    let params = block_node.parameters()?;
    let block_params = params.as_block_parameters_node()?;
    let parameters = block_params.parameters()?;
    let requireds = parameters.requireds();
    let first = requireds.iter().next()?;
    let req_param = first.as_required_parameter_node()?;
    Some(req_param.name().as_slice().to_vec())
}

/// Check if the truthy branch (then-clause) of an if node returns just the
/// block parameter variable. Returns true if the truthy branch is a simple
/// local variable read matching `param_name`.
fn truthy_branch_returns_param(
    source: &SourceFile,
    if_node: &ruby_prism::IfNode<'_>,
    param_name: &[u8],
) -> bool {
    let then_body = match if_node.statements() {
        Some(s) => s,
        None => return false,
    };

    let stmts: Vec<_> = then_body.body().iter().collect();
    if stmts.len() != 1 {
        return false;
    }

    if let Some(lvar) = stmts[0].as_local_variable_read_node() {
        let _ = source; // suppress unused warning
        return lvar.name().as_slice() == param_name;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapCompactWithConditionalBlock, "cops/style/map_compact_with_conditional_block");
}
