use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE};

pub struct ZipWithoutBlock;

impl Cop for ZipWithoutBlock {
    fn name(&self) -> &'static str {
        "Performance/ZipWithoutBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Look for CallNode .map or .collect with a block
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_none() {
            return;
        }

        let method_name = call.name().as_slice();
        if method_name != b"map" && method_name != b"collect" {
            return;
        }

        // Must have a block (not a block argument like &method)
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(bn) => bn,
            None => return,
        };

        // Get the block parameter name
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

        let requireds = param_list.requireds();
        if requireds.len() != 1 {
            return;
        }

        let first_param = match requireds.iter().next() {
            Some(p) => p,
            None => return,
        };

        let param_name = match first_param.as_required_parameter_node() {
            Some(rp) => rp.name(),
            None => return,
        };

        // Body must be a single array literal containing only one element:
        // the same variable as the block parameter
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_stmts = stmts.body();
        if body_stmts.len() != 1 {
            return;
        }

        let stmt = match body_stmts.iter().next() {
            Some(s) => s,
            None => return,
        };

        let array = match stmt.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let elements = array.elements();
        if elements.len() != 1 {
            return;
        }

        let elem = match elements.iter().next() {
            Some(e) => e,
            None => return,
        };

        let local_var = match elem.as_local_variable_read_node() {
            Some(lv) => lv,
            None => return,
        };

        if local_var.name().as_slice() != param_name.as_slice() {
            return;
        }

        // Offense spans from the method name selector to the end of the block
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return,
        };

        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `zip` without a block argument instead.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ZipWithoutBlock, "cops/performance/zip_without_block");
}
