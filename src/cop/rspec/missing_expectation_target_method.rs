use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct MissingExpectationTargetMethod;

impl Cop for MissingExpectationTargetMethod {
    fn name(&self) -> &'static str {
        "RSpec/MissingExpectationTargetMethod"
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
        // Look for expect(x).something or is_expected.something
        // where something is not .to / .not_to / .to_not
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Skip if it's one of the valid target methods
        if method_name == b"to" || method_name == b"not_to" || method_name == b"to_not" {
            return;
        }

        // Check if receiver is `expect(...)` or `expect { ... }` or `is_expected`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_expect = if let Some(recv_call) = recv.as_call_node() {
            let recv_name = recv_call.name().as_slice();
            (recv_name == b"expect" || recv_name == b"is_expected")
                && recv_call.receiver().is_none()
        } else {
            false
        };

        if !is_expect {
            return;
        }

        let loc = call.message_loc();
        let (line, column) = match loc {
            Some(l) => source.offset_to_line_col(l.start_offset()),
            None => {
                let loc = call.location();
                source.offset_to_line_col(loc.start_offset())
            }
        };

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `.to`, `.not_to` or `.to_not` to set an expectation.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MissingExpectationTargetMethod, "cops/rspec/missing_expectation_target_method");
}
