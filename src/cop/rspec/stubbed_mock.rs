use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE};

pub struct StubbedMock;

/// Flags `expect(foo).to receive(:bar).and_return(...)` and similar patterns
/// where a message expectation also configures a response.
/// Prefer `allow` over `expect` when configuring a response.
///
/// Matches RuboCop's behavior: only checks the TOP-LEVEL matcher node against
/// specific patterns. Intermediate methods like `.twice`/`.once` break the chain.
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
        &[BLOCK_ARGUMENT_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, HASH_NODE, KEYWORD_HASH_NODE]
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

        let method_name = call.name().as_slice();

        // We need this to be a `.to` call
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        // Check the argument (the matcher expression)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        let matcher = &arg_list[0];

        // Check for the RuboCop patterns:
        // 1. matcher_with_configured_response: outermost call is and_return/etc. on a message_expectation
        // 2. matcher_with_hash: receive_messages(hash) or receive_message_chain(... hash)
        // 3. matcher_with_blockpass: receive/receive_message_chain with &block_pass
        // 4. Block on the matcher (receive(:foo) { 'bar' }) — block attached to CallNode
        // 5. Block on the .to call (without explicit parens, block goes to .to)
        let has_response = is_matcher_with_configured_response(matcher)
            || is_matcher_with_hash(matcher)
            || is_matcher_with_blockpass(matcher)
            || is_matcher_with_block(matcher)
            || is_to_call_with_block_on_message_expectation(&call, matcher);

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

/// Check if a node is a "message_expectation" per RuboCop:
///   receive(...)                             — (send nil? :receive ...)
///   receive_message_chain(...)               — (send nil? :receive_message_chain ...)
///   receive(:foo).with(...)                  — (send (send nil? :receive ...) :with ...)
///
/// Note: receive_message_chain(...).with(...) does NOT match.
/// Note: receive(:foo).twice does NOT match — only `.with` is allowed after `receive`.
fn is_message_expectation(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let name = call.name().as_slice();

    if call.receiver().is_none() {
        // Direct call: receive(...) or receive_message_chain(...)
        return name == b"receive" || name == b"receive_message_chain";
    }

    // Chained: only receive(...).with(...)
    if name == b"with" {
        if let Some(recv) = call.receiver() {
            if let Some(recv_call) = recv.as_call_node() {
                if recv_call.receiver().is_none() && recv_call.name().as_slice() == b"receive" {
                    return true;
                }
            }
        }
    }

    false
}

/// Pattern 1: (send #message_expectation? #configured_response? _)
/// e.g. receive(:foo).and_return('bar')
/// e.g. receive(:foo).with(42).and_return('bar')
/// NOT: receive(:foo).twice.and_return('bar') — .twice breaks message_expectation
/// NOT: receive(:foo).and_return('bar').once — .once is the outermost, not a configured_response
/// NOT: receive(:foo).and_call_original — no argument, pattern requires exactly one `_`
fn is_matcher_with_configured_response(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let name = call.name().as_slice();
    if !is_configured_response(name) {
        return false;
    }
    // RuboCop pattern (send #message_expectation? #configured_response? _) requires
    // exactly one argument after the method name. Methods like and_call_original
    // with no arguments don't match.
    let has_args = call.arguments().map_or(false, |a| {
        let args: Vec<_> = a.arguments().iter().collect();
        !args.is_empty()
    });
    if !has_args {
        return false;
    }
    if let Some(recv) = call.receiver() {
        return is_message_expectation(&recv);
    }
    false
}

fn is_configured_response(name: &[u8]) -> bool {
    matches!(
        name,
        b"and_return" | b"and_raise" | b"and_throw" | b"and_yield"
            | b"and_call_original" | b"and_wrap_original"
    )
}

/// Pattern 3: receive_messages(hash) or receive_message_chain(... hash)
/// e.g. receive_messages(foo: 'bar') or receive_message_chain(:foo, bar: 'baz')
fn is_matcher_with_hash(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if call.receiver().is_some() {
        return false;
    }

    let name = call.name().as_slice();
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };
    let arg_list: Vec<_> = args.arguments().iter().collect();
    if arg_list.is_empty() {
        return false;
    }

    match name {
        b"receive_messages" => {
            // receive_messages(hash) — first arg is a hash
            arg_list[0].as_hash_node().is_some() || arg_list[0].as_keyword_hash_node().is_some()
        }
        b"receive_message_chain" => {
            // receive_message_chain(:foo, bar: 'baz') — last arg is a hash
            let last = &arg_list[arg_list.len() - 1];
            last.as_hash_node().is_some() || last.as_keyword_hash_node().is_some()
        }
        _ => false,
    }
}

/// Pattern 4: receive/receive_message_chain with &block_pass
/// e.g. receive(:foo, &canned), receive_message_chain(:foo, &canned),
///      receive(:foo).with('bar', &canned)
fn is_matcher_with_blockpass(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    // Check for block_pass on this call
    if let Some(block) = call.block() {
        if block.as_block_argument_node().is_some() {
            let name = call.name().as_slice();
            if call.receiver().is_none() {
                // receive(:foo, &canned) or receive_message_chain(:foo, &canned)
                if name == b"receive" || name == b"receive_message_chain" {
                    return true;
                }
            }
            // receive(:foo).with('bar', &canned)
            if name == b"with" {
                if let Some(recv) = call.receiver() {
                    if let Some(recv_call) = recv.as_call_node() {
                        if recv_call.receiver().is_none() && recv_call.name().as_slice() == b"receive" {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// Check if the matcher itself has a block (e.g. the CallNode for receive has a block attached).
/// In Prism, `receive(:foo) { 'bar' }` with explicit parens would have the block on the receive CallNode.
fn is_matcher_with_block(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if !is_message_expectation(node) {
        return false;
    }

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
    }

    false
}

/// Check if the .to call itself has a block and the matcher is a message_expectation.
/// This handles `expect(foo).to receive(:bar) { 'baz' }` without explicit parens,
/// where the block is attached to the .to call rather than the receive call.
fn is_to_call_with_block_on_message_expectation(
    to_call: &ruby_prism::CallNode<'_>,
    matcher: &ruby_prism::Node<'_>,
) -> bool {
    // The matcher must be a message_expectation
    if !is_message_expectation(matcher) {
        return false;
    }

    if let Some(block) = to_call.block() {
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
        // Block argument (&canned) on .to — this is a blockpass on .to
        if block.as_block_argument_node().is_some() {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StubbedMock, "cops/rspec/stubbed_mock");
}
