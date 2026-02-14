use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for receive(:method) { |&block| block.call ... }
        // The node structure: CallNode(receive) with a BlockNode
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // The call could be `receive(:foo)` or `receive(:foo).with(...)` etc.
        // We need to find the block that has a `&block` parameter and body is only `block.call`
        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        // Check if the block has a block parameter (&block)
        let params = match block.parameters() {
            Some(p) => match p.as_block_parameters_node() {
                Some(bp) => bp,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let inner_params = match params.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Must have a block parameter (&block)
        let block_param = match inner_params.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_param_name = block_param.name();
        let block_param_bytes = match block_param_name {
            Some(n) => n.as_slice().to_vec(),
            None => return Vec::new(),
        };

        // Check that the body is only block.call statements
        let body = match block.body() {
            Some(b) => match b.as_statements_node() {
                Some(s) => s,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let stmts: Vec<_> = body.body().iter().collect();
        if stmts.is_empty() {
            return Vec::new();
        }

        // Every statement must be `block.call` or `block.call(args)`
        for stmt in &stmts {
            let stmt_call = match stmt.as_call_node() {
                Some(c) => c,
                None => return Vec::new(),
            };

            if stmt_call.name().as_slice() != b"call" {
                return Vec::new();
            }

            // Receiver must be the block parameter
            let recv = match stmt_call.receiver() {
                Some(r) => r,
                None => return Vec::new(),
            };

            if let Some(recv_call) = recv.as_call_node() {
                if recv_call.name().as_slice() != block_param_bytes.as_slice() {
                    return Vec::new();
                }
                if recv_call.receiver().is_some() {
                    return Vec::new();
                }
            } else if let Some(local) = recv.as_local_variable_read_node() {
                if local.name().as_slice() != block_param_bytes.as_slice() {
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }
        }

        // Check that the outer call chain includes `receive`
        if !call_chain_includes_receive(call) {
            return Vec::new();
        }

        let loc = block.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `.and_yield`.".to_string(),
        )]
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
