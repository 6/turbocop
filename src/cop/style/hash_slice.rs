use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE, STRING_NODE, SYMBOL_NODE};

pub struct HashSlice;

impl Cop for HashSlice {
    fn name(&self) -> &'static str {
        "Style/HashSlice"
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
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_bytes = call.name().as_slice();

        // Only handle select, filter
        if method_bytes != b"select" && method_bytes != b"filter" {
            return Vec::new();
        }

        if call.receiver().is_none() {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let params = match block_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return Vec::new(),
        };

        let parameters = match block_params.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let requireds: Vec<_> = parameters.requireds().iter().collect();
        if requireds.len() != 2 {
            return Vec::new();
        }

        let key_param = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return Vec::new(),
        };
        let key_name = key_param.name().as_slice();

        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return Vec::new();
        }

        if let Some(cmp_call) = body_nodes[0].as_call_node() {
            let cmp_method = cmp_call.name().as_slice();

            // Check for k == :sym pattern (select -> slice)
            if cmp_method == b"==" {
                let cmp_recv = match cmp_call.receiver() {
                    Some(r) => r,
                    None => return Vec::new(),
                };

                let cmp_args = match cmp_call.arguments() {
                    Some(a) => a,
                    None => return Vec::new(),
                };

                let cmp_arg_list: Vec<_> = cmp_args.arguments().iter().collect();
                if cmp_arg_list.len() != 1 {
                    return Vec::new();
                }

                let value_node = if let Some(lvar) = cmp_recv.as_local_variable_read_node() {
                    if lvar.name().as_slice() == key_name {
                        &cmp_arg_list[0]
                    } else {
                        return Vec::new();
                    }
                } else if let Some(lvar) = cmp_arg_list[0].as_local_variable_read_node() {
                    if lvar.name().as_slice() == key_name {
                        &cmp_recv
                    } else {
                        return Vec::new();
                    }
                } else {
                    return Vec::new();
                };

                if value_node.as_symbol_node().is_none() && value_node.as_string_node().is_none() {
                    return Vec::new();
                }

                let value_src = &source.as_bytes()[value_node.location().start_offset()..value_node.location().end_offset()];
                let value_str = String::from_utf8_lossy(value_src);

                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `slice({})` instead.", value_str),
                )];
            }

            // Check for array.include?(k) pattern (select -> slice(*array))
            if cmp_method == b"include?" {
                let include_recv = match cmp_call.receiver() {
                    Some(r) => r,
                    None => return Vec::new(),
                };

                let include_args = match cmp_call.arguments() {
                    Some(a) => a,
                    None => return Vec::new(),
                };

                let include_arg_list: Vec<_> = include_args.arguments().iter().collect();
                if include_arg_list.len() != 1 {
                    return Vec::new();
                }

                // The argument to include? must be the key param
                let is_key_arg = include_arg_list[0]
                    .as_local_variable_read_node()
                    .map(|lv| lv.name().as_slice() == key_name)
                    .unwrap_or(false);

                if !is_key_arg {
                    return Vec::new();
                }

                let recv_src = &source.as_bytes()[include_recv.location().start_offset()..include_recv.location().end_offset()];
                let recv_str = String::from_utf8_lossy(recv_src);

                let loc = call.message_loc().unwrap_or_else(|| call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use `slice(*{})` instead.", recv_str),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HashSlice, "cops/style/hash_slice");
}
