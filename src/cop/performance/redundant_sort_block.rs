use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantSortBlock;

impl Cop for RedundantSortBlock {
    fn name(&self) -> &'static str {
        "Performance/RedundantSortBlock"
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

        if call.name().as_slice() != b"sort" {
            return Vec::new();
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return Vec::new();
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Check if the block is `{ |a, b| a <=> b }` — the redundant default sort
        // The block should be a BlockNode with a body that is a single CallNode for `<=>`
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Must have exactly 2 block parameters
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return Vec::new(),
        };

        let param_list = match block_params.parameters() {
            Some(pl) => pl,
            None => return Vec::new(),
        };

        let requireds: Vec<_> = param_list.requireds().iter().collect();
        if requireds.len() != 2 {
            return Vec::new();
        }

        // Get the parameter names
        let param_a = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return Vec::new(),
        };
        let param_b = match requireds[1].as_required_parameter_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let name_a = param_a.name().as_slice();
        let name_b = param_b.name().as_slice();

        // Body should be a single `a <=> b` call
        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let statements = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let stmts: Vec<_> = statements.body().iter().collect();
        if stmts.len() != 1 {
            return Vec::new();
        }

        let spaceship_call = match stmts[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if spaceship_call.name().as_slice() != b"<=>" {
            return Vec::new();
        }

        // Check receiver is param_a and argument is param_b (a <=> b, not b <=> a)
        let recv = match spaceship_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_name = match recv.as_local_variable_read_node() {
            Some(lv) => lv.name().as_slice(),
            None => return Vec::new(),
        };

        let args = match spaceship_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_nodes: Vec<_> = args.arguments().iter().collect();
        if arg_nodes.len() != 1 {
            return Vec::new();
        }

        let arg_name = match arg_nodes[0].as_local_variable_read_node() {
            Some(lv) => lv.name().as_slice(),
            None => return Vec::new(),
        };

        // Check that it's `a <=> b` (same order as parameters), making it redundant
        if recv_name != name_a || arg_name != name_b {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `sort` instead of `sort { |a, b| a <=> b }`.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSortBlock, "cops/performance/redundant_sort_block");
}
