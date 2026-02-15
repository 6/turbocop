use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RootJoinChain;

impl Cop for RootJoinChain {
    fn name(&self) -> &'static str {
        "Rails/RootJoinChain"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        if call.name().as_slice() != b"join" {
            return Vec::new();
        }

        // Don't flag if this join is itself a receiver of another join (wait for the outermost)
        // We can't check parent directly, so this cop fires on every .join.join chain.
        // Instead, walk the receiver chain down to find if it's a .join on Rails.root/public_path.

        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // The receiver should be another .join call
        let recv_call = match recv.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if recv_call.name().as_slice() != b"join" {
            return Vec::new();
        }

        // Walk the chain to find if it originates from Rails.root or Rails.public_path
        if !chain_starts_with_rails_root(recv_call) {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use a single `join` with multiple arguments instead of chaining.".to_string(),
        )]
    }
}

/// Walk down a chain of .join calls to see if the bottom is Rails.root or Rails.public_path
fn chain_starts_with_rails_root(call: ruby_prism::CallNode<'_>) -> bool {
    let recv = match call.receiver() {
        Some(r) => r,
        None => return false,
    };

    // If the receiver is another .join, recurse
    if let Some(recv_call) = recv.as_call_node() {
        if recv_call.name().as_slice() == b"join" {
            return chain_starts_with_rails_root(recv_call);
        }
        // Check if this is Rails.root or Rails.public_path
        if recv_call.name().as_slice() == b"root" || recv_call.name().as_slice() == b"public_path" {
            if let Some(r) = recv_call.receiver() {
                return util::constant_name(&r) == Some(b"Rails");
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RootJoinChain, "cops/rails/root_join_chain");
}
