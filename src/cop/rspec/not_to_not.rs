use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct NotToNot;

impl Cop for NotToNot {
    fn name(&self) -> &'static str {
        "RSpec/NotToNot"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "not_to");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        if enforced_style == "not_to" {
            // Flag `to_not`
            if method_name != b"to_not" {
                return;
            }
        } else {
            // Flag `not_to`
            if method_name != b"not_to" {
                return;
            }
        }

        // Verify receiver is an expect call
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // The receiver should be an expect(...) call or an expect block
        let is_expect_receiver = if let Some(recv_call) = receiver.as_call_node() {
            recv_call.name().as_slice() == b"expect" && recv_call.receiver().is_none()
        } else {
            false
        };

        if !is_expect_receiver {
            return;
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        let (preferred, flagged) = if enforced_style == "not_to" {
            ("not_to", "to_not")
        } else {
            ("to_not", "not_to")
        };

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Prefer `{preferred}` over `{flagged}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NotToNot, "cops/rspec/not_to_not");
}
