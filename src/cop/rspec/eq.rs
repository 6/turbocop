use crate::cop::node_type::CALL_NODE;
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Eq;

impl Cop for Eq {
    fn name(&self) -> &'static str {
        "RSpec/Eq"
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
        // Look for `be == value` pattern
        // This appears as a call to `==` with receiver being the `be` call
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"==" {
            return;
        }

        // The receiver should be a `be` call
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if recv_call.name().as_slice() != b"be" {
            return;
        }

        // `be` should have no arguments (bare `be`)
        let has_args = recv_call
            .arguments()
            .map(|a| a.arguments().iter().count() > 0)
            .unwrap_or(false);
        if has_args {
            return;
        }

        let loc = recv_call.location();
        let end_loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let (_, end_column) = source.offset_to_line_col(end_loc.start_offset());
        // The offense covers "be ==" - the be call + == call name
        let _ = end_column;
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `eq` instead of `be ==` to compare objects.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Eq, "cops/rspec/eq");
}
