use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE};

pub struct CompareWithBlock;

impl Cop for CompareWithBlock {
    fn name(&self) -> &'static str {
        "Performance/CompareWithBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE]
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

        if call.name().as_slice() != b"sort" {
            return;
        }

        if call.receiver().is_none() {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Must have exactly 2 block parameters
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };

        let param_list = match block_params.parameters() {
            Some(pl) => pl,
            None => return,
        };

        let requireds: Vec<_> = param_list.requireds().iter().collect();
        if requireds.len() != 2 {
            return;
        }

        let param_a = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };
        let param_b = match requireds[1].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };

        let name_a = param_a.name().as_slice();
        let name_b = param_b.name().as_slice();

        // Body should be a single `a.method <=> b.method` call
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let statements = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let stmts: Vec<_> = statements.body().iter().collect();
        if stmts.len() != 1 {
            return;
        }

        let spaceship_call = match stmts[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        if spaceship_call.name().as_slice() != b"<=>" {
            return;
        }

        // Check receiver is a method call on param_a: a.method
        let recv = match spaceship_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let recv_receiver = match recv_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_var = match recv_receiver.as_local_variable_read_node() {
            Some(lv) => lv,
            None => return,
        };

        if recv_var.name().as_slice() != name_a {
            return;
        }

        // Check argument is a method call on param_b: b.method
        let args = match spaceship_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_nodes: Vec<_> = args.arguments().iter().collect();
        if arg_nodes.len() != 1 {
            return;
        }

        let arg_call = match arg_nodes[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        let arg_receiver = match arg_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let arg_var = match arg_receiver.as_local_variable_read_node() {
            Some(lv) => lv,
            None => return,
        };

        if arg_var.name().as_slice() != name_b {
            return;
        }

        // Both should call the same method
        let method_a = recv_call.name().as_slice();
        let method_b = arg_call.name().as_slice();
        if method_a != method_b {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, "Use `sort_by(&:method)` instead of `sort { |a, b| a.method <=> b.method }`.".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CompareWithBlock, "cops/performance/compare_with_block");
}
