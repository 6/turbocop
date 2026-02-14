use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MessageChain;

impl Cop for MessageChain {
    fn name(&self) -> &'static str {
        "RSpec/MessageChain"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check for `receive_message_chain` (receiverless)
        if method_name == b"receive_message_chain" && call.receiver().is_none() {
            let loc = call.location();
            let msg_loc = call.message_loc().unwrap_or(loc);
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Avoid stubbing using `receive_message_chain`.".to_string(),
            )];
        }

        // Check for old `stub_chain` syntax (has receiver)
        if method_name == b"stub_chain" && call.receiver().is_some() {
            let msg_loc = match call.message_loc() {
                Some(l) => l,
                None => return Vec::new(),
            };
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Avoid stubbing using `stub_chain`.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MessageChain, "cops/rspec/message_chain");
}
