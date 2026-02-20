use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE, INTERPOLATED_REGULAR_EXPRESSION_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE, REGULAR_EXPRESSION_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE};

pub struct SelectByRegexp;

impl SelectByRegexp {
    fn is_regexp(node: &ruby_prism::Node<'_>) -> bool {
        node.as_regular_expression_node().is_some() || node.as_interpolated_regular_expression_node().is_some()
    }

    fn is_local_var_named(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
        if let Some(lvar) = node.as_local_variable_read_node() {
            return lvar.name().as_slice() == name;
        }
        false
    }

    fn check_block_body(body: &ruby_prism::Node<'_>, block_arg_name: &[u8]) -> bool {
        if let Some(call) = body.as_call_node() {
            let name = call.name();
            let name_bytes = name.as_slice();
            if matches!(name_bytes, b"match?" | b"=~" | b"!~") {
                if let Some(receiver) = call.receiver() {
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 {
                            let recv_is_var = Self::is_local_var_named(&receiver, block_arg_name);
                            let arg_is_var = Self::is_local_var_named(&arg_list[0], block_arg_name);
                            let recv_is_re = Self::is_regexp(&receiver);
                            let arg_is_re = Self::is_regexp(&arg_list[0]);

                            return (recv_is_var && arg_is_re) || (recv_is_re && arg_is_var);
                        }
                    }
                }
            }
        }
        false
    }

    fn is_hash_receiver(node: &ruby_prism::Node<'_>) -> bool {
        if node.as_hash_node().is_some() || node.as_keyword_hash_node().is_some() {
            return true;
        }
        if let Some(call) = node.as_call_node() {
            let name = call.name();
            let name_bytes = name.as_slice();
            if matches!(name_bytes, b"to_h" | b"to_hash") {
                return true;
            }
            if matches!(name_bytes, b"new" | b"[]") {
                if let Some(recv) = call.receiver() {
                    if let Some(cr) = recv.as_constant_read_node() {
                        if cr.name().as_slice() == b"Hash" {
                            return true;
                        }
                    }
                    if let Some(cp) = recv.as_constant_path_node() {
                        if cp.location().as_slice().ends_with(b"Hash") {
                            return true;
                        }
                    }
                }
            }
        }
        if let Some(cr) = node.as_constant_read_node() {
            if cr.name().as_slice() == b"ENV" {
                return true;
            }
        }
        if let Some(cp) = node.as_constant_path_node() {
            if cp.location().as_slice().ends_with(b"ENV") {
                return true;
            }
        }
        false
    }
}

impl Cop for SelectByRegexp {
    fn name(&self) -> &'static str {
        "Style/SelectByRegexp"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, HASH_NODE, INTERPOLATED_REGULAR_EXPRESSION_NODE, KEYWORD_HASH_NODE, LOCAL_VARIABLE_READ_NODE, REGULAR_EXPRESSION_NODE, REQUIRED_PARAMETER_NODE, STATEMENTS_NODE]
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
        // We check the CallNode; its block() gives us the BlockNode
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Must be select, filter, find_all, or reject
        if !matches!(method_bytes, b"select" | b"filter" | b"find_all" | b"reject") {
            return;
        }

        // Must not be called on a hash-like receiver
        if let Some(receiver) = call.receiver() {
            if Self::is_hash_receiver(&receiver) {
                return;
            }
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

        // Get block parameters - must have exactly one
        let block_params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_params_node = match block_params.as_block_parameters_node() {
            Some(p) => p,
            None => return,
        };

        let inner_params = match block_params_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let requireds: Vec<_> = inner_params.requireds().into_iter().collect();
        if requireds.len() != 1 {
            return;
        }

        let block_arg_name = match requireds[0].as_required_parameter_node() {
            Some(req) => req.name().as_slice().to_vec(),
            None => return,
        };

        // Block body must be a single expression that matches regexp
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        if let Some(stmts) = body.as_statements_node() {
            let body_nodes: Vec<_> = stmts.body().into_iter().collect();
            if body_nodes.len() != 1 {
                return;
            }

            if !Self::check_block_body(&body_nodes[0], &block_arg_name) {
                return;
            }
        } else {
            return;
        }

        let replacement = match method_bytes {
            b"select" | b"filter" | b"find_all" => "grep",
            b"reject" => "grep_v",
            _ => return,
        };

        let method_str = std::str::from_utf8(method_bytes).unwrap_or("select");
        // Report on the whole call including block
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{}` to `{}` with a regexp match.", replacement, method_str),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SelectByRegexp, "cops/style/select_by_regexp");
}
