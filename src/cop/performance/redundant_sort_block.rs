use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"sort" {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        // Check if the block is `{ |a, b| a <=> b }` — the redundant default sort
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        // Determine the expected parameter names based on block type
        let (name_a, name_b) = if let Some(block_params) = params.as_block_parameters_node() {
            // Regular block: { |a, b| a <=> b }
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

            (
                param_a.name().as_slice().to_vec(),
                param_b.name().as_slice().to_vec(),
            )
        } else if let Some(numbered) = params.as_numbered_parameters_node() {
            // Numbered params: { _1 <=> _2 }
            if numbered.maximum() < 2 {
                return;
            }
            (b"_1".to_vec(), b"_2".to_vec())
        } else {
            return;
        };

        // Body should be a single `a <=> b` call
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

        // Check receiver is param_a and argument is param_b (a <=> b, not b <=> a)
        let recv = match spaceship_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_name = match recv.as_local_variable_read_node() {
            Some(lv) => lv.name().as_slice(),
            None => return,
        };

        let args = match spaceship_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_nodes: Vec<_> = args.arguments().iter().collect();
        if arg_nodes.len() != 1 {
            return;
        }

        let arg_name = match arg_nodes[0].as_local_variable_read_node() {
            Some(lv) => lv.name().as_slice(),
            None => return,
        };

        // Check that it's `a <=> b` (same order as parameters), making it redundant
        if recv_name != name_a.as_slice() || arg_name != name_b.as_slice() {
            return;
        }

        // Report at the method selector (sort), not the entire expression
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return,
        };
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `sort` instead of `sort { |a, b| a <=> b }`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSortBlock, "cops/performance/redundant_sort_block");
}
