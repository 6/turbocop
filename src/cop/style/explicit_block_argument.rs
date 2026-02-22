use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE,
    STATEMENTS_NODE, YIELD_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ExplicitBlockArgument;

impl Cop for ExplicitBlockArgument {
    fn name(&self) -> &'static str {
        "Style/ExplicitBlockArgument"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            LOCAL_VARIABLE_READ_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
            YIELD_NODE,
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
        // Look for block nodes where the body is just `yield` with the same args
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Must have a body
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_nodes: Vec<_> = stmts.body().into_iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        // Check if the single statement is a yield
        let yield_node = match body_nodes[0].as_yield_node() {
            Some(y) => y,
            None => return,
        };

        // Check that the block has parameters and the yield passes them through
        let block_params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        // For simplicity, check that the yield args match the block params
        let param_node = match block_params.as_block_parameters_node() {
            Some(p) => p,
            None => return,
        };

        let params = match param_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let required_params: Vec<_> = params.requireds().into_iter().collect();
        if required_params.is_empty() {
            return;
        }

        let yield_args = match yield_node.arguments() {
            Some(a) => a,
            None => return,
        };

        let yield_arg_list: Vec<_> = yield_args.arguments().into_iter().collect();

        // Simple check: same number of args as block params
        if yield_arg_list.len() != required_params.len() {
            return;
        }

        // Check that each yield arg is a local variable read matching the block param
        for (param, arg) in required_params.iter().zip(yield_arg_list.iter()) {
            let param_name = if let Some(rp) = param.as_required_parameter_node() {
                rp.name()
            } else {
                return;
            };

            let arg_name = if let Some(lv) = arg.as_local_variable_read_node() {
                lv.name()
            } else {
                return;
            };

            if param_name.as_slice() != arg_name.as_slice() {
                return;
            }
        }

        // The block just passes through args to yield — suggest using explicit &block
        let loc = block_node.location();
        // Report at the call node that has the block
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Consider using explicit block argument in the surrounding method's signature over `yield`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExplicitBlockArgument, "cops/style/explicit_block_argument");
}
