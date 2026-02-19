use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};

pub struct UnspecifiedException;

/// Detects `raise_error` / `raise_exception` without an exception class argument
/// when used with `.to` (not `.not_to`).
impl Cop for UnspecifiedException {
    fn name(&self) -> &'static str {
        "RSpec/UnspecifiedException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE]
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

        let method_name = call.name().as_slice();

        // Look for `.to` calls (not `.not_to` or `.to_not` — those are fine without args)
        if method_name != b"to" {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Walk the argument's call chain to find the root matcher
        let root = match find_root_call(&arg_list[0]) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let root_name = root.name().as_slice();
        if root_name != b"raise_error" && root_name != b"raise_exception" {
            return Vec::new();
        }

        // Must have no receiver (standalone matcher call)
        if root.receiver().is_some() {
            return Vec::new();
        }

        // Must have no arguments (specifying an exception class)
        if root.arguments().is_some() {
            return Vec::new();
        }

        // Must have no block (braces: raise_error { |e| ... })
        if root.block().is_some() {
            return Vec::new();
        }

        // Also check if the `.to` call has a block with arguments.
        // `expect { }.to raise_error do |e| ... end` — the do/end block attaches
        // to `.to`, not to `raise_error`. If `.to`'s block has parameters,
        // the exception IS being captured via the block argument.
        if let Some(to_block) = call.block() {
            if let Some(block_node) = to_block.as_block_node() {
                if block_node.parameters().is_some() {
                    return Vec::new();
                }
            }
        }

        let loc = root.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Specify the exception being captured.".to_string(),
        )]
    }
}

/// Walk a call chain down to the root (receiverless) call.
fn find_root_call<'a>(node: &ruby_prism::Node<'a>) -> Option<ruby_prism::CallNode<'a>> {
    let mut current = node.as_call_node()?;
    loop {
        match current.receiver() {
            None => return Some(current),
            Some(recv) => {
                current = recv.as_call_node()?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnspecifiedException, "cops/rspec/unspecified_exception");
}
