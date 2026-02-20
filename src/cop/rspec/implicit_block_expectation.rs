use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct ImplicitBlockExpectation;

impl Cop for ImplicitBlockExpectation {
    fn name(&self) -> &'static str {
        "RSpec/ImplicitBlockExpectation"
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
        // Flag `is_expected` when sibling `subject` contains a lambda/proc
        // This is a simplified check: we flag `is_expected` inside example blocks
        // that are siblings of a subject with a lambda/proc body.
        //
        // Simplified approach: detect `is_expected` call with no receiver and flag it
        // when the call has a `.to` chain that contains change/raise_error block matchers.
        //
        // Actually, from the vendor spec the pattern is:
        //   subject { -> { boom } }
        //   it { is_expected.to change { something } }
        // We flag `is_expected` when used as implicit block expectation.
        // The simplest detection: `is_expected.to` followed by a block-expecting matcher.

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Look for `.to` or `.not_to` or `.to_not` calls
        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        // Check if receiver is `is_expected`
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if recv_call.receiver().is_some() {
            return;
        }

        if recv_call.name().as_slice() != b"is_expected" {
            return;
        }

        // Check if the matcher argument is a block-expecting matcher (change, raise_error, etc.)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        for arg in args.arguments().iter() {
            if has_block_matcher(&arg) {
                let loc = recv_call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid implicit block expectations.".to_string(),
                ));
            }
            break;
        }

    }
}

/// Check if a node is a block-expecting matcher (change, raise_error, raise_exception, throw_symbol).
fn has_block_matcher(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        // Check the innermost method in the chain
        if let Some(recv) = call.receiver() {
            if has_block_matcher(&recv) {
                return true;
            }
        }
        if call.receiver().is_none() || call.block().is_some() {
            if name == b"change"
                || name == b"raise_error"
                || name == b"raise_exception"
                || name == b"throw_symbol"
                || name == b"output"
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ImplicitBlockExpectation, "cops/rspec/implicit_block_expectation");
}
