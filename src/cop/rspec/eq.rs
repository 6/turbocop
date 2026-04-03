use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
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
        // Look for matcher runner calls where the matcher arg is `be == value`:
        // `expect(foo).to be == 42`
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"to" && method_name != b"not_to" && method_name != b"to_not" {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let matcher = match args.arguments().iter().next() {
            Some(m) => m,
            None => return,
        };

        let eq_call = match matcher.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if eq_call.name().as_slice() != b"==" {
            return;
        }

        // The `==` receiver should be a bare `be` call.
        let recv = match eq_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let be_call = match recv.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if be_call.name().as_slice() != b"be" {
            return;
        }

        // `be` must have no receiver (bare `be` call), matching RuboCop's `(send nil? :be)`.
        // In legacy `.should.be == value` syntax, `be` has a receiver (the should proxy).
        if be_call.receiver().is_some() {
            return;
        }

        // `be` should have no arguments (bare `be`)
        let has_args = be_call
            .arguments()
            .map(|a| a.arguments().iter().count() > 0)
            .unwrap_or(false);
        if has_args {
            return;
        }

        let loc = be_call.location();
        let end_loc = eq_call.location();
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
