use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SharedExamples;

/// Methods that accept shared example titles.
const SHARED_EXAMPLE_METHODS: &[&[u8]] = &[
    b"it_behaves_like",
    b"it_should_behave_like",
    b"shared_examples",
    b"shared_examples_for",
    b"include_examples",
];

impl Cop for SharedExamples {
    fn name(&self) -> &'static str {
        "RSpec/SharedExamples"
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

        // Check for RSpec.shared_examples as well
        let is_shared = if call.receiver().is_some() {
            if let Some(recv) = call.receiver() {
                if let Some(c) = recv.as_constant_read_node() {
                    c.name().as_slice() == b"RSpec"
                        && (method_name == b"shared_examples"
                            || method_name == b"shared_examples_for")
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            SHARED_EXAMPLE_METHODS
                .iter()
                .any(|m| method_name == *m)
        };

        if !is_shared {
            return Vec::new();
        }

        // Get the first argument — should be a string (default enforced style)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        for arg in args.arguments().iter() {
            // Only flag symbol arguments (default style enforces strings)
            if let Some(sym) = arg.as_symbol_node() {
                let sym_name = std::str::from_utf8(sym.unescaped()).unwrap_or("");
                let title = sym_name.replace('_', " ");
                let loc = sym.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Prefer '{}' over `:{sym_name}` to titleize shared examples.",
                        title
                    ),
                )];
            }
            // Stop at first positional arg (skip keyword hashes)
            if arg.as_keyword_hash_node().is_none() {
                break;
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SharedExamples, "cops/rspec/shared_examples");
}
