use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, STATEMENTS_NODE};

pub struct Yield;

/// Flags `receive(:method) { |&block| block.call }` — should use `.and_yield` instead.
impl Cop for Yield {
    fn name(&self) -> &'static str {
        "RSpec/Yield"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Look for receive(:method) { |&block| block.call ... }
        // The node structure: CallNode(receive) with a BlockNode
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // The call could be `receive(:foo)` or `receive(:foo).with(...)` etc.
        // We need to find the block that has a `&block` parameter and body is only `block.call`
        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        // Check if the block has a block parameter (&block)
        let params = match block.parameters() {
            Some(p) => match p.as_block_parameters_node() {
                Some(bp) => bp,
                None => return,
            },
            None => return,
        };

        let inner_params = match params.parameters() {
            Some(p) => p,
            None => return,
        };

        // Must have a block parameter (&block)
        let block_param = match inner_params.block() {
            Some(b) => b,
            None => return,
        };

        let block_param_name = block_param.name();
        let block_param_bytes = match block_param_name {
            Some(n) => n.as_slice().to_vec(),
            None => return,
        };

        // Check that the body is only block.call statements
        let body = match block.body() {
            Some(b) => match b.as_statements_node() {
                Some(s) => s,
                None => return,
            },
            None => return,
        };

        let stmts: Vec<_> = body.body().iter().collect();
        if stmts.is_empty() {
            return;
        }

        // Every statement must be `block.call` or `block.call(args)`
        for stmt in &stmts {
            let stmt_call = match stmt.as_call_node() {
                Some(c) => c,
                None => return,
            };

            if stmt_call.name().as_slice() != b"call" {
                return;
            }

            // Receiver must be the block parameter
            let recv = match stmt_call.receiver() {
                Some(r) => r,
                None => return,
            };

            if let Some(recv_call) = recv.as_call_node() {
                if recv_call.name().as_slice() != block_param_bytes.as_slice() {
                    return;
                }
                if recv_call.receiver().is_some() {
                    return;
                }
            } else if let Some(local) = recv.as_local_variable_read_node() {
                if local.name().as_slice() != block_param_bytes.as_slice() {
                    return;
                }
            } else {
                return;
            }
        }

        // Check that the outer call chain includes `receive`
        if !call_chain_includes_receive(call) {
            return;
        }

        let loc = block.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `.and_yield`.".to_string(),
        ));
    }
}

fn call_chain_includes_receive(call: ruby_prism::CallNode<'_>) -> bool {
    // Check if the call or any receiver in the chain is `receive`
    let name = call.name().as_slice();
    if name == b"receive" {
        return true;
    }

    if let Some(recv) = call.receiver() {
        if let Some(recv_call) = recv.as_call_node() {
            return call_chain_includes_receive(recv_call);
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Yield, "cops/rspec/yield_cop");
}
