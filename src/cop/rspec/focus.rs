use crate::cop::shared::constant_predicates;
use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_focused};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

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

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = FocusVisitor {
            cop: self,
            source,
            diagnostics,
            def_depth: 0,
        };
        visitor.visit(&parse_result.node());
    }
}

struct FocusVisitor<'a> {
    cop: &'a Focus,
    source: &'a SourceFile,
    diagnostics: &'a mut Vec<Diagnostic>,
    /// Depth inside method definitions (def/defs). When > 0, skip flagging.
    /// Matches RuboCop's `node.each_ancestor(:any_def).any?`.
    def_depth: u32,
}

impl<'pr> Visit<'pr> for FocusVisitor<'_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.def_depth += 1;
        ruby_prism::visit_def_node(self, node);
        self.def_depth -= 1;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Skip focus detection inside method definitions
        if self.def_depth == 0 {
            self.check_focus(node);
        }
        ruby_prism::visit_call_node(self, node);
    }
}

impl FocusVisitor<'_> {
    fn check_focus(&mut self, call: &ruby_prism::CallNode<'_>) {
        let method_name = call.name().as_slice();

        // Check for f-prefixed methods (fit, fdescribe, fcontext, etc.)
        // Also matches bare `focus` calls from Ruby 3.1+ shorthand `focus:`.
        if is_rspec_focused(method_name) {
            // Only flag receiverless calls. Calls like analyzer.fit(x) have a
            // receiver and are not RSpec focus.
            if call.receiver().is_none() && !is_chained_call(self.source, call) {
                let loc = call.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(Diagnostic {
                    path: self.source.path_str().to_string(),
                    location: crate::diagnostic::Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.cop.name().to_string(),
                    message: "Focused spec found.".to_string(),
                    corrected: false,
                });
            }
            return;
        }

        // Check for focus metadata on RSpec methods
        let is_rspec_method = if call.receiver().is_none() {
            is_focusable_method(method_name)
        } else if let Some(recv) = call.receiver() {
            constant_predicates::constant_short_name(&recv).is_some_and(|n| n == b"RSpec")
                && (method_name == b"describe" || method_name == b"fdescribe")
        } else {
            false
        };

        if !is_rspec_method {
            return;
        }

        // Check for focus: true or :focus in arguments
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                // Check for :focus symbol in arguments
                if let Some(sym) = arg.as_symbol_node() {
                    if sym.unescaped() == b"focus" {
                        let loc = sym.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Focused spec found.".to_string(),
                        ));
                        return;
                    }
                }
                // Check for focus: true in hash arguments
                if let Some(hash) = arg.as_keyword_hash_node() {
                    for elem in hash.elements().iter() {
                        if let Some(pair) = elem.as_assoc_node() {
                            if let Some(key) = pair.key().as_symbol_node() {
                                if key.unescaped() == b"focus"
                                    && pair.value().as_true_node().is_some()
                                {
                                    let start = key.location().start_offset();
                                    let (line, column) = self.source.offset_to_line_col(start);
                                    self.diagnostics.push(self.cop.diagnostic(
                                        self.source,
                                        line,
                                        column,
                                        "Focused spec found.".to_string(),
                                    ));
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Focus, "cops/rspec/focus");
}
