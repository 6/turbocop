use crate::cop::node_type::CALL_NODE;
use crate::cop::util::{self, RSPEC_DEFAULT_INCLUDE, has_rspec_focus_metadata, is_rspec_focused};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/Focus: Checks if examples are focused.
///
/// FN investigation (2026-03): 7 FNs all from Ruby 3.1+ keyword argument
/// shorthand `focus:` (equivalent to `focus: focus`). In Parser gem AST, the
/// value of `focus:` shorthand is `(send nil? :focus)` — a bare method call.
/// RuboCop's `focused_block?` pattern matches ANY `(send nil? :focus)` that is
/// not chained and not inside a method definition. In Prism, the shorthand
/// produces `ImplicitNode { CallNode(focus) }`, and the explicit `focus: focus`
/// produces a bare `CallNode(focus)`. Both are visited by the walker.
///
/// Root cause: the cop previously required `call.block().is_some()` for focused
/// method detection, which excluded the blockless implicit/explicit `focus` calls
/// from shorthand keyword args.
///
/// Fix: removed the block requirement for focused methods. Added a chaining check
/// (source text peek for `.` / `&.` after the call) to match RuboCop's
/// `node.chained?` guard, preventing FPs on patterns like `fit.id`.
pub struct Focus;

/// All RSpec methods that can have focus metadata or be f-prefixed.
const RSPEC_FOCUSABLE: &[&str] = &[
    "describe",
    "context",
    "feature",
    "example_group",
    "xdescribe",
    "xcontext",
    "xfeature",
    "it",
    "specify",
    "example",
    "scenario",
    "xit",
    "xspecify",
    "xexample",
    "xscenario",
    "pending",
    "skip",
    "shared_examples",
    "shared_examples_for",
    "shared_context",
];

fn is_focusable_method(name: &[u8]) -> bool {
    let s = std::str::from_utf8(name).unwrap_or("");
    RSPEC_FOCUSABLE.contains(&s)
}

/// Check if a call node is "chained" — i.e., used as the receiver of another
/// method call. Detects `.` or `&.` immediately following the call expression
/// in the source text (after optional whitespace on the same line).
fn is_chained_call(source: &SourceFile, call: &ruby_prism::CallNode<'_>) -> bool {
    let end = call.location().end_offset();
    let src = source.as_bytes();
    let mut i = end;
    // Skip horizontal whitespace only (not newlines)
    while i < src.len() && (src[i] == b' ' || src[i] == b'\t') {
        i += 1;
    }
    if i < src.len() && src[i] == b'.' {
        return true;
    }
    if i + 1 < src.len() && src[i] == b'&' && src[i + 1] == b'.' {
        return true;
    }
    false
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

        // Check for f-prefixed methods (fit, fdescribe, fcontext, etc.)
        // Also matches bare `focus` calls from Ruby 3.1+ shorthand `focus:` (which
        // desugars to `focus: focus` where the value is `(send nil? :focus)`).
        if is_rspec_focused(method_name) {
            // Only flag receiverless calls (fit, fdescribe, focus, etc.).
            // Calls like analyzer.fit(x) have a receiver and are not RSpec focus.
            if call.receiver().is_none() && !is_chained_call(source, &call) {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(Diagnostic {
                    path: source.path_str().to_string(),
                    location: crate::diagnostic::Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Focused spec found.".to_string(),

                    corrected: false,
                });
            }
            return;
        }

        // Check for focus metadata on RSpec methods
        // Must be a recognized RSpec method OR RSpec.describe / ::RSpec.describe
        let is_rspec_method = if call.receiver().is_none() {
            is_focusable_method(method_name)
        } else if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec")
                && (method_name == b"describe" || method_name == b"fdescribe")
        } else {
            false
        };

        if !is_rspec_method {
            return;
        }

        // Check for focus: true or :focus in arguments
        if let Some((line, col, _offset, _len)) = has_rspec_focus_metadata(source, node) {
            diagnostics.push(Diagnostic {
                path: source.path_str().to_string(),
                location: crate::diagnostic::Location { line, column: col },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Focused spec found.".to_string(),

                corrected: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Focus, "cops/rspec/focus");
}
