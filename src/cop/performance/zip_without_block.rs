use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ZipWithoutBlock;

impl Cop for ZipWithoutBlock {
    fn name(&self) -> &'static str {
        "Performance/ZipWithoutBlock"
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
        // Look for CallNode .map or .collect with a block
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.receiver().is_none() {
            return Vec::new();
        }

        let method_name = call.name().as_slice();
        if method_name != b"map" && method_name != b"collect" {
            return Vec::new();
        }

        // Must have a block (not a block argument like &method)
        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(bn) => bn,
            None => return Vec::new(),
        };

        // Get the block parameter name
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

        let requireds = param_list.requireds();
        if requireds.len() != 1 {
            return Vec::new();
        }

        let first_param = match requireds.iter().next() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let param_name = match first_param.as_required_parameter_node() {
            Some(rp) => rp.name(),
            None => return Vec::new(),
        };

        // Body must be a single array literal containing only one element:
        // the same variable as the block parameter
        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_stmts = stmts.body();
        if body_stmts.len() != 1 {
            return Vec::new();
        }

        let stmt = match body_stmts.iter().next() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let array = match stmt.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let elements = array.elements();
        if elements.len() != 1 {
            return Vec::new();
        }

        let elem = match elements.iter().next() {
            Some(e) => e,
            None => return Vec::new(),
        };

        let local_var = match elem.as_local_variable_read_node() {
            Some(lv) => lv,
            None => return Vec::new(),
        };

        if local_var.name().as_slice() != param_name.as_slice() {
            return Vec::new();
        }

        // Offense spans from the method name selector to the end of the block
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `zip` without a block argument instead.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ZipWithoutBlock, "cops/performance/zip_without_block");
}
