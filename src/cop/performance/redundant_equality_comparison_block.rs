use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantEqualityComparisonBlock;

const FLAGGED_METHODS: &[&[u8]] = &[
    b"all?", b"any?", b"count", b"detect", b"filter", b"find",
    b"find_index", b"include?", b"max", b"min", b"none?", b"one?",
    b"reject", b"select",
];

impl Cop for RedundantEqualityComparisonBlock {
    fn name(&self) -> &'static str {
        "Performance/RedundantEqualityComparisonBlock"
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
        if !FLAGGED_METHODS.iter().any(|m| *m == method_name) {
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

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Must have exactly 1 block parameter
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
        if requireds.len() != 1 {
            return Vec::new();
        }

        let param = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let param_name = param.name().as_slice();

        // Body should be a single equality comparison: x == value or value == x
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

        let eq_call = match stmts[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if eq_call.name().as_slice() != b"==" {
            return Vec::new();
        }

        // Check that one side of == is the block parameter
        let recv = match eq_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let args = match eq_call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_nodes: Vec<_> = args.arguments().iter().collect();
        if arg_nodes.len() != 1 {
            return Vec::new();
        }

        let recv_is_param = recv
            .as_local_variable_read_node()
            .is_some_and(|lv| lv.name().as_slice() == param_name);

        let arg_is_param = arg_nodes[0]
            .as_local_variable_read_node()
            .is_some_and(|lv| lv.name().as_slice() == param_name);

        if !recv_is_param && !arg_is_param {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `grep` or `===` comparison instead of block with `==`.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantEqualityComparisonBlock, "cops/performance/redundant_equality_comparison_block");
}
