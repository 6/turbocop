use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE};

pub struct HashExcept;

impl Cop for HashExcept {
    fn name(&self) -> &'static str {
        "Style/HashExcept"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE]
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

        let method_bytes = call.name().as_slice();

        // Only handle reject, select, filter
        if method_bytes != b"reject" && method_bytes != b"select" && method_bytes != b"filter" {
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

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Must have exactly 2 block parameters (|k, v|)
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };

        let parameters = match block_params.parameters() {
            Some(p) => p,
            None => return,
        };

        let requireds: Vec<_> = parameters.requireds().iter().collect();
        if requireds.len() != 2 {
            return;
        }

        // Get the key parameter name
        let key_param = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };
        let key_name = key_param.name().as_slice();

        // Check the block body for a simple comparison pattern
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

        // Check for k == :sym pattern (reject) or k != :sym pattern (select/filter)
        if let Some(cmp_call) = expr.as_call_node() {
            let cmp_method = cmp_call.name().as_slice();

            // For reject: k == :sym -> except(:sym)
            // For select/filter: k != :sym -> except(:sym)
            let is_matching = (method_bytes == b"reject" && cmp_method == b"==")
                || ((method_bytes == b"select" || method_bytes == b"filter") && cmp_method == b"!=");

            if !is_matching {
                return;
            }

            let cmp_recv = match cmp_call.receiver() {
                Some(r) => r,
                None => return,
            };

            let cmp_args = match cmp_call.arguments() {
                Some(a) => a,
                None => return,
            };

            let cmp_arg_list: Vec<_> = cmp_args.arguments().iter().collect();
            if cmp_arg_list.len() != 1 {
                return;
            }

            // One side must be the key param, other must be a literal
            let (is_key_left, value_node) = if let Some(lvar) = cmp_recv.as_local_variable_read_node() {
                if lvar.name().as_slice() == key_name {
                    (true, &cmp_arg_list[0])
                } else {
                    return;
                }
            } else if let Some(lvar) = cmp_arg_list[0].as_local_variable_read_node() {
                if lvar.name().as_slice() == key_name {
                    (false, &cmp_recv)
                } else {
                    return;
                }
            } else {
                return;
            };

            let _ = is_key_left;

            // Value must be a symbol or string literal
            let is_sym_or_str = value_node.as_symbol_node().is_some()
                || value_node.as_string_node().is_some();

            if !is_sym_or_str {
                return;
            }

            let value_src = &source.as_bytes()[value_node.location().start_offset()..value_node.location().end_offset()];
            let value_str = String::from_utf8_lossy(value_src);

            let loc = call.message_loc().unwrap_or_else(|| call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Use `except({})` instead.", value_str),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashExcept, "cops/style/hash_except");
}
