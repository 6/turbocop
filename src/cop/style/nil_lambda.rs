use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, LAMBDA_NODE, NIL_NODE, RETURN_NODE, STATEMENTS_NODE};

pub struct NilLambda;

impl NilLambda {
    /// Check if a body node is just `nil`, `return nil`, `next nil`, or `break nil`
    fn is_nil_return(node: &ruby_prism::Node<'_>) -> bool {
        // Direct nil
        if node.as_nil_node().is_some() {
            return true;
        }
        // `return nil`
        if let Some(ret) = node.as_return_node() {
            if let Some(args) = ret.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                return arg_list.len() == 1 && arg_list[0].as_nil_node().is_some();
            }
        }
        false
    }

    fn is_lambda_or_proc_call(call: &ruby_prism::CallNode<'_>) -> Option<&'static str> {
        let name = call.name();
        let name_bytes = name.as_slice();

        // Check for `lambda` or `proc` bare calls (no receiver)
        if call.receiver().is_none() {
            if name_bytes == b"lambda" {
                return Some("lambda");
            }
            if name_bytes == b"proc" {
                return Some("proc");
            }
        }

        // Check for `Proc.new`
        if name_bytes == b"new" {
            if let Some(recv) = call.receiver() {
                if let Some(cr) = recv.as_constant_read_node() {
                    if cr.name().as_slice() == b"Proc" {
                        return Some("proc");
                    }
                }
                if let Some(cp) = recv.as_constant_path_node() {
                    if cp.location().as_slice().ends_with(b"Proc") {
                        return Some("proc");
                    }
                }
            }
        }

        None
    }

    fn check_block_body(body: &ruby_prism::Node<'_>) -> bool {
        if let Some(stmts) = body.as_statements_node() {
            let body_nodes: Vec<_> = stmts.body().into_iter().collect();
            if body_nodes.len() == 1 && Self::is_nil_return(&body_nodes[0]) {
                return true;
            }
        }
        false
    }
}

impl Cop for NilLambda {
    fn name(&self) -> &'static str {
        "Style/NilLambda"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, LAMBDA_NODE, NIL_NODE, RETURN_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check lambda node: `-> { nil }`
        if let Some(lambda) = node.as_lambda_node() {
            if let Some(body) = lambda.body() {
                if Self::check_block_body(&body) {
                    let loc = lambda.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use an empty lambda instead of always returning nil.".to_string(),
                    ));
                }
            }
        }

        // Check call node with block: `proc { nil }`, `lambda { nil }`, `Proc.new { nil }`
        if let Some(call) = node.as_call_node() {
            if let Some(type_name) = Self::is_lambda_or_proc_call(&call) {
                if let Some(block) = call.block() {
                    if let Some(block_node) = block.as_block_node() {
                        if let Some(body) = block_node.body() {
                            if Self::check_block_body(&body) {
                                // Report on the whole expression including the block
                                let loc = node.location();
                                let (line, column) = source.offset_to_line_col(loc.start_offset());
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!("Use an empty {} instead of always returning nil.", type_name),
                                ));
                            }
                        }
                    }
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NilLambda, "cops/style/nil_lambda");
}
