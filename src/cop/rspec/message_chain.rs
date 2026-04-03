use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Matches RuboCop's `RSpec/MessageChain` send-based behavior for
/// `receive_message_chain` and `stub_chain`.
///
/// 2026-03-29 FN fix: corpus examples in
/// `rspec-mocks/spec/rspec/mocks/any_instance/message_chains_spec.rb` used a
/// receiverless `stub_chain` helper provided by `let`. The previous
/// implementation only flagged `stub_chain` when the call had an explicit
/// receiver (`foo.stub_chain(...)`), so all seven offenses were missed.
///
/// Fix: treat every Prism `CALL_NODE` named `stub_chain` as an offense, matching
/// RuboCop's `RESTRICT_ON_SEND`. True local variables remain excluded because
/// Prism parses them as `local_variable_read_node`, not `call_node`.
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for `receive_message_chain` (receiverless)
        if method_name == b"receive_message_chain" && call.receiver().is_none() {
            let loc = call.location();
            let msg_loc = call.message_loc().unwrap_or(loc);
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Avoid stubbing using `receive_message_chain`.".to_string(),
            ));
        }

        // Check for old `stub_chain` syntax. RuboCop flags any send named
        // `stub_chain`, including receiverless helper calls from `let`.
        if method_name == b"stub_chain" {
            let msg_loc = match call.message_loc() {
                Some(l) => l,
                None => return,
            };
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Avoid stubbing using `stub_chain`.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MessageChain, "cops/rspec/message_chain");
}
