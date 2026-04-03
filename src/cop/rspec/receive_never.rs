use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-29): FN=5 in rspec/rspec.
/// The old matcher only handled `.to receive(...).never` when `.never` was the
/// direct first matcher argument, so it missed:
/// - negative runners written as `.not_to` / `.to_not`
/// - extra chaining after `.never`, e.g. `.never.and_return(1)`
/// - helper wrappers like `wrapped.not_to receive(:foo).never`, where the
///   runner receiver is not a literal `expect(...)` call in the same node
///
/// Fix: inspect runner calls directly, search their matcher arguments
/// recursively for `receive(...).never`, and still skip explicit `allow` /
/// `allow_any_instance_of` receivers to preserve the cop's allowance
/// exemption.
pub struct ReceiveNever;

impl Cop for ReceiveNever {
    fn name(&self) -> &'static str {
        "RSpec/ReceiveNever"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let runner_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let runner_name = runner_call.name().as_slice();
        if runner_name != b"to" && runner_name != b"not_to" && runner_name != b"to_not" {
            return;
        }

        if runner_name == b"to" {
            let recv = match runner_call.receiver() {
                Some(r) => r,
                None => return,
            };

            if !is_expect_call(&recv) {
                return;
            }
        } else if runner_call
            .receiver()
            .is_some_and(|recv| is_allow_call(&recv))
        {
            return;
        }

        let args = match runner_call.arguments() {
            Some(args) => args,
            None => return,
        };

        let never_call = match args.arguments().iter().find_map(find_never_call) {
            Some(call) => call,
            None => return,
        };

        let loc = never_call
            .message_loc()
            .unwrap_or_else(|| never_call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `not_to receive` instead of `never`.".to_string(),
        ));
    }
}

/// Check if a node is an expect-like call (expect, expect_any_instance_of, is_expected).
/// Returns false for allow-like calls.
fn is_expect_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        // Expect-like calls
        if name == b"expect" || name == b"expect_any_instance_of" || name == b"is_expected" {
            return true;
        }
        // Allow-like calls should not match
        if name == b"allow" || name == b"allow_any_instance_of" {
            return false;
        }
    }
    false
}

fn is_allow_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if name == b"allow" || name == b"allow_any_instance_of" {
            return true;
        }
    }
    false
}

/// Check if the node subtree contains a receiverless `receive(...)` matcher.
fn contains_receive_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"receive" && call.receiver().is_none() {
            return true;
        }
        if let Some(recv) = call.receiver() {
            if contains_receive_call(&recv) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if contains_receive_call(&arg) {
                    return true;
                }
            }
        }
    }
    false
}

fn find_never_call<'a>(node: ruby_prism::Node<'a>) -> Option<ruby_prism::CallNode<'a>> {
    let call = node.as_call_node()?;
    if call.name().as_slice() == b"never" && contains_receive_call(&node) {
        return Some(call);
    }
    if let Some(recv) = call.receiver() {
        if let Some(never_call) = find_never_call(recv) {
            return Some(never_call);
        }
    }
    if let Some(args) = call.arguments() {
        for arg in args.arguments().iter() {
            if let Some(never_call) = find_never_call(arg) {
                return Some(never_call);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReceiveNever, "cops/rspec/receive_never");
}
