use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE};

pub struct RedundantSortBy;

impl Cop for RedundantSortBy {
    fn name(&self) -> &'static str {
        "Style/RedundantSortBy"
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
    ) -> Vec<Diagnostic> {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be `sort_by` method
        if call_node.name().as_slice() != b"sort_by" {
            return Vec::new();
        }

        // Must have a receiver
        if call_node.receiver().is_none() {
            return Vec::new();
        }

        // Must have a block
        let block = match call_node.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Block must have exactly one parameter and body is just that parameter
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let bp = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return Vec::new(),
        };

        let inner_params = match bp.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Must have exactly one required parameter
        let requireds: Vec<_> = inner_params.requireds().iter().collect();
        if requireds.len() != 1 {
            return Vec::new();
        }

        // No other params
        if !inner_params.optionals().is_empty()
            || inner_params.rest().is_some()
            || !inner_params.posts().is_empty()
            || !inner_params.keywords().is_empty()
            || inner_params.keyword_rest().is_some()
            || inner_params.block().is_some()
        {
            return Vec::new();
        }

        // Get the param name
        let param_node = &requireds[0];
        let param_name = match param_node.as_required_parameter_node() {
            Some(p) => p.name(),
            None => return Vec::new(),
        };

        // Body must be a single statement that is a local variable read of the same name
        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => {
                // Try direct local variable read
                if let Some(lvar) = body.as_local_variable_read_node() {
                    if lvar.name().as_slice() != param_name.as_slice() {
                        return Vec::new();
                    }
                } else {
                    return Vec::new();
                }
                // The body is just the variable - this is a match
                let msg_loc = call_node.message_loc().unwrap_or_else(|| call_node.location());
                let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
                let var_name = std::str::from_utf8(param_name.as_slice()).unwrap_or("x");
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `sort` instead of `sort_by {{ |{}| {} }}`.", var_name, var_name),
                )];
            }
        };

        let stmts_body: Vec<_> = stmts.body().iter().collect();
        if stmts_body.len() != 1 {
            return Vec::new();
        }

        let body_node = &stmts_body[0];
        let lvar = match body_node.as_local_variable_read_node() {
            Some(l) => l,
            None => return Vec::new(),
        };

        if lvar.name().as_slice() != param_name.as_slice() {
            return Vec::new();
        }

        let msg_loc = call_node.message_loc().unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        let var_name = std::str::from_utf8(param_name.as_slice()).unwrap_or("x");
        vec![self.diagnostic(
            source,
            line,
            column,
            format!("Use `sort` instead of `sort_by {{ |{}| {} }}`.", var_name, var_name),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSortBy, "cops/style/redundant_sort_by");
}
