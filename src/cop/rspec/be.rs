use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct Be;

impl Cop for Be {
    fn name(&self) -> &'static str {
        "RSpec/Be"
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
    ) -> Vec<Diagnostic> {
        // Look for `expect(...).to be` / `expect(...).not_to be` / `expect(...).to_not be`
        // The `be` is a CallNode with receiver being another CallNode (`.to`/`.not_to`/`.to_not`)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return Vec::new();
        }

        // Check that the argument is a bare `be` call (no args, no chain)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let first_arg = &arg_list[0];
        let be_call = match first_arg.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if be_call.name().as_slice() != b"be" {
            return Vec::new();
        }

        // Must have no receiver (standalone `be`, not `foo.be`)
        if be_call.receiver().is_some() {
            return Vec::new();
        }

        // Must have no arguments
        if be_call.arguments().is_some() {
            return Vec::new();
        }

        let loc = be_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Don't use `be` without an argument.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Be, "cops/rspec/be");
}
