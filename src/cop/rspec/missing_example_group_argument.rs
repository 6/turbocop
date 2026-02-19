use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct MissingExampleGroupArgument;

const EXAMPLE_GROUP_METHODS: &[&[u8]] = &[
    b"describe", b"context", b"feature", b"example_group",
];

impl Cop for MissingExampleGroupArgument {
    fn name(&self) -> &'static str {
        "RSpec/MissingExampleGroupArgument"
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
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        if !EXAMPLE_GROUP_METHODS.contains(&method_name) {
            return;
        }

        // Must have a block
        if call.block().is_none() {
            return;
        }

        // Must be receiverless or RSpec.describe / ::RSpec.describe
        let is_rspec_call = if call.receiver().is_none() {
            true
        } else if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec")
        } else {
            false
        };

        if !is_rspec_call {
            return;
        }

        // Must have no arguments (or only keyword/metadata args, but no positional)
        if call.arguments().is_some() {
            return;
        }

        let method_str = std::str::from_utf8(method_name).unwrap_or("describe");

        // Flag the entire call up to the block
        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("The first argument to `{method_str}` should not be empty."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MissingExampleGroupArgument, "cops/rspec/missing_example_group_argument");
}
