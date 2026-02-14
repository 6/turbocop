use crate::cop::util::{has_rspec_focus_metadata, is_rspec_focused, RSPEC_DEFAULT_INCLUDE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Focus;

/// All RSpec methods that can have focus metadata or be f-prefixed.
const RSPEC_FOCUSABLE: &[&str] = &[
    "describe", "context", "feature", "example_group",
    "xdescribe", "xcontext", "xfeature",
    "it", "specify", "example", "scenario",
    "xit", "xspecify", "xexample", "xscenario",
    "pending", "skip",
    "shared_examples", "shared_examples_for", "shared_context",
];

fn is_focusable_method(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    RSPEC_FOCUSABLE.contains(&s)
}

impl Cop for Focus {
    fn name(&self) -> &'static str {
        "RSpec/Focus"
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

        // Check for f-prefixed methods (fit, fdescribe, fcontext, etc.)
        if is_rspec_focused(method_name) {
            // Must have a block to be an RSpec call (not just a method call inside def)
            if call.block().is_some() {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: crate::diagnostic::Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Focused spec found.".to_string(),
                }];
            }
            return Vec::new();
        }

        // Check for focus metadata on RSpec methods
        // Must be a recognized RSpec method OR RSpec.describe
        let is_rspec_method = if call.receiver().is_none() {
            is_focusable_method(method_name)
        } else if let Some(recv) = call.receiver() {
            if let Some(recv_const) = recv.as_constant_read_node() {
                recv_const.name().as_slice() == b"RSpec"
                    && (method_name == b"describe" || method_name == b"fdescribe")
            } else {
                false
            }
        } else {
            false
        };

        if !is_rspec_method {
            return Vec::new();
        }

        // Check for focus: true or :focus in arguments
        if let Some((line, col, _offset, _len)) = has_rspec_focus_metadata(source, node) {
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: crate::diagnostic::Location { line, column: col },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Focused spec found.".to_string(),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Focus, "cops/rspec/focus");
}
