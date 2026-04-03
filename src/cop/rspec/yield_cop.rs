use crate::cop::shared::node_type::{
    BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, LOCAL_VARIABLE_READ_NODE, STATEMENTS_NODE,
};
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=21, FN=11.
///
/// FP=21: Root cause was that the cop flagged blocks with extra positional params
/// alongside &block (e.g., `|options, &block|`). RuboCop's pattern `(args (blockarg $_))`
/// only matches when &block is the sole parameter. Fixed by checking that the block
/// has no required, optional, rest, or keyword params — only the block param.
///
/// FN=11: Root cause was a function name mismatch — `call_chain_includes_receive` was
/// called but the actual function was renamed to `call_chain_or_args_includes_receive`
/// (which checks `receive` in both receiver chain and arguments, needed for `do...end`
/// blocks where the block attaches to `.to` rather than `receive`). Fixed 2026-03-18.
///
/// ## Corpus investigation (2026-03-20)
///
/// FN=2: Both from honeybadger-ruby. Pattern: `expect(x).to receive(:foo).with(bar) do |&block|`
/// where `receive` is chained with `.with(...)`. The `do...end` block attaches to `.to`,
/// so `receive(:foo).with(bar)` becomes an argument to `.to`. The argument-checking code
/// only looked at the top-level argument name (which was `with`, not `receive`). Fixed by
/// recursively walking the argument's receiver chain via `call_chain_includes_receive_shallow`.
///
/// ## Corpus investigation (2026-03-25)
///
/// Corpus oracle reported FP=4, FN=0.
///
/// FP=4: All from `block&.call` (safe navigation). RuboCop only matches `block.call`
/// (regular dot call), not `block&.call`. Fixed by checking `call_operator_loc` for `&.`
/// and skipping safe navigation calls.
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
        &[
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            LOCAL_VARIABLE_READ_NODE,
            STATEMENTS_NODE,
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

        // Must have a block parameter (&block) and NO other parameters.
        // RuboCop's pattern `(args (blockarg $_))` only matches when &block is the sole param.
        let block_param = match inner_params.block() {
            Some(b) => b,
            None => return,
        };

        // Check that there are no other parameters besides the block param
        if inner_params.requireds().iter().count() > 0
            || inner_params.optionals().iter().count() > 0
            || inner_params.rest().is_some()
            || inner_params.keywords().iter().count() > 0
            || inner_params.keyword_rest().is_some()
        {
            return;
        }

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

        // Every statement must be `block.call` or `block.call(args)` (regular dot, not safe navigation)
        for stmt in &stmts {
            let stmt_call = match stmt.as_call_node() {
                Some(c) => c,
                None => return,
            };

            if stmt_call.name().as_slice() != b"call" {
                return;
            }

            // RuboCop only matches `block.call` (regular dot), not `block&.call` (safe navigation).
            if stmt_call
                .call_operator_loc()
                .is_some_and(|loc| loc.as_slice() == b"&.")
            {
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
        if !call_chain_or_args_includes_receive(&call) {
            return;
        }

        let loc = block.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, "Use `.and_yield`.".to_string()));
    }
}

fn call_chain_or_args_includes_receive(call: &ruby_prism::CallNode<'_>) -> bool {
    // Check if the call itself is `receive`
    let name = call.name().as_slice();
    if name == b"receive" {
        return true;
    }

    // Check arguments for `receive` (handles `do...end` blocks on `.to` where
    // `receive(:method)` is an argument rather than receiver).
    // The argument may be chained, e.g. `receive(:foo).with(bar)`, so we
    // recursively walk the argument's receiver chain too.
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if let Some(arg_call) = arg.as_call_node() {
                if call_chain_includes_receive_shallow(&arg_call) {
                    return true;
                }
            }
        }
    }

    // Walk receiver chain
    if let Some(recv) = call.receiver() {
        if let Some(recv_call) = recv.as_call_node() {
            return call_chain_or_args_includes_receive(&recv_call);
        }
    }

    false
}

/// Walk a call chain checking if any call in the chain is `receive`.
fn call_chain_includes_receive_shallow(call: &ruby_prism::CallNode<'_>) -> bool {
    if call.name().as_slice() == b"receive" {
        return true;
    }
    if let Some(recv) = call.receiver() {
        if let Some(recv_call) = recv.as_call_node() {
            return call_chain_includes_receive_shallow(&recv_call);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Yield, "cops/rspec/yield_cop");
}
