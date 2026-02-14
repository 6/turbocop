use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct NamedSubject;

/// Flags usage of bare `subject` inside examples/hooks when it should be named.
/// Default behavior: always flag `subject` references in examples/hooks
/// (the user should use `subject(:name)` and then reference by name).
impl Cop for NamedSubject {
    fn name(&self) -> &'static str {
        "RSpec/NamedSubject"
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
        // Look for bare `subject` calls (no receiver, no arguments, no block)
        // that appear to be references to the test subject
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"subject" {
            return Vec::new();
        }

        // Must have no receiver
        if call.receiver().is_some() {
            return Vec::new();
        }

        // Must have no block (subject { ... } is a declaration, not a reference)
        if call.block().is_some() {
            return Vec::new();
        }

        // Must have no arguments (subject(:name) is a declaration)
        if call.arguments().is_some() {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Name your test subject if you need to reference it explicitly.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NamedSubject, "cops/rspec/named_subject");
}
