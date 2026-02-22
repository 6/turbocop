use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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
        // Look for .to / .to_not / .not_to calls (the matcher dispatch)
        let to_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let to_method = to_call.name().as_slice();
        if to_method != b"to" {
            return;
        }

        // The receiver of .to should be an expect-like call
        let recv = match to_call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !is_expect_call(&recv) {
            return;
        }

        // The argument to .to should be a chain ending in .never with receive
        let args = match to_call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check if the first argument is a chain that ends with .never on a receive
        let arg_call = match arg_list[0].as_call_node() {
            Some(c) => c,
            None => return,
        };

        if arg_call.name().as_slice() != b"never" {
            return;
        }

        // Check that the chain contains receive
        let recv_of_never = match arg_call.receiver() {
            Some(r) => r,
            None => return,
        };

        if !has_receive_in_chain(&recv_of_never) {
            return;
        }

        let loc = arg_call
            .message_loc()
            .unwrap_or_else(|| arg_call.location());
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

/// Check if the chain contains a `receive` call somewhere.
fn has_receive_in_chain(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if name == b"receive" {
            return true;
        }
        if let Some(recv) = call.receiver() {
            return has_receive_in_chain(&recv);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReceiveNever, "cops/rspec/receive_never");
}
