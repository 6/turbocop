use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE};

pub struct StubbedMock;

/// Flags `expect(foo).to receive(:bar).and_return(...)` and similar patterns
/// where a message expectation also configures a response.
/// Prefer `allow` over `expect` when configuring a response.
impl Cop for StubbedMock {
    fn name(&self) -> &'static str {
        "RSpec/StubbedMock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // We need this to be a `.to` call (or a chain ending in `.to`)
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        // Check the argument for receive/receive_messages/receive_message_chain
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let matcher_name = root_matcher_name(&arg_list[0]);
        let has_response = match matcher_name.as_deref() {
            Some(b"receive") => {
                // Check if the chain includes and_return/and_yield/etc.
                has_response_in_chain(&arg_list[0]) || has_block_response(&call)
            }
            Some(b"receive_messages") | Some(b"receive_message_chain") => true,
            _ => return,
        };

        if !has_response {
            return;
        }

        // Get the receiver of `.to` — should be expect(...), is_expected, etc.
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let recv_name = recv_call.name().as_slice();
        let recv_loc = recv_call.location();
        let (line, column) = source.offset_to_line_col(recv_loc.start_offset());

        match recv_name {
            b"expect" if recv_call.receiver().is_none() => {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `allow` over `expect` when configuring a response.".to_string(),
                ));
            }
            b"expect_any_instance_of" if recv_call.receiver().is_none() => {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `allow_any_instance_of` over `expect_any_instance_of` when configuring a response.".to_string(),
                ));
            }
            b"is_expected" if recv_call.receiver().is_none() => {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer `allow(subject)` over `is_expected` when configuring a response.".to_string(),
                ));
            }
            _ => {}
        }
    }
}

fn root_matcher_name<'a>(node: &ruby_prism::Node<'a>) -> Option<&'a [u8]> {
    let call = node.as_call_node()?;
    let mut current = call;
    loop {
        if current.receiver().is_none() {
            return Some(current.name().as_slice());
        }
        let recv = current.receiver()?;
        current = recv.as_call_node()?;
    }
}

fn has_response_in_chain(node: &ruby_prism::Node<'_>) -> bool {
    // Walk the call chain from the argument looking for and_return, etc.
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    let mut current = call;
    loop {
        let method = current.name().as_slice();
        if matches!(
            method,
            b"and_return" | b"and_yield" | b"and_raise" | b"and_throw"
        ) {
            return true;
        }
        // Check for block on receive (not the outermost block-pass)
        // e.g., receive(:bar) { 'value' } — but when there's a block on receive itself
        // That would mean the block is on the receive, which is actually passed as a block_pass arg
        // to receive. This is handled by has_block_response.

        let recv = match current.receiver() {
            Some(r) => r,
            None => return false,
        };
        current = match recv.as_call_node() {
            Some(c) => c,
            None => return false,
        };
    }
}

fn has_block_response(to_call: &ruby_prism::CallNode<'_>) -> bool {
    // Check if the .to call itself has a block
    if check_block(to_call) {
        return true;
    }
    // Also check the argument chain (in Ruby, `receive(:bar) { ... }` has
    // the block on `receive`, not on `.to`)
    if let Some(args) = to_call.arguments() {
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if !arg_list.is_empty() {
            if has_block_in_arg_chain(&arg_list[0]) {
                return true;
            }
        }
    }
    false
}

fn check_block(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(block) = call.block() {
        if let Some(bn) = block.as_block_node() {
            // Block with params like |x| is dynamic, not a stubbed response
            if let Some(params) = bn.parameters() {
                if let Some(bp) = params.as_block_parameters_node() {
                    if let Some(p) = bp.parameters() {
                        let req: Vec<_> = p.requireds().iter().collect();
                        if !req.is_empty() {
                            return false;
                        }
                    }
                }
            }
            return true;
        }
        if block.as_block_argument_node().is_some() {
            return true;
        }
    }
    false
}

fn has_block_in_arg_chain(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let mut current = call;
    loop {
        if check_block(&current) {
            return true;
        }
        let recv = match current.receiver() {
            Some(r) => r,
            None => return false,
        };
        current = match recv.as_call_node() {
            Some(c) => c,
            None => return false,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StubbedMock, "cops/rspec/stubbed_mock");
}
