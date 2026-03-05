use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// RSpec/VoidExpect: flags `expect(...)` or `expect { ... }` calls that are not
/// chained with `.to`, `.not_to`, or `.to_not`.
///
/// Investigation (32 FP, 6804 FN): The cop was missing the critical `inside_example?`
/// check. RuboCop only flags void expects INSIDE example blocks (it, specify, example,
/// scenario, and their f/x variants), but nitrocop was flagging ALL statement-level
/// `expect` calls regardless of context. This caused FPs on `expect` calls in
/// non-example contexts (e.g., helper methods, shared_context blocks) and massive FNs
/// because the cop's overly-broad matching was likely being suppressed by config or
/// the check_source approach missed nested example blocks.
///
/// Fix: Added `in_example` depth tracking to the visitor. The visitor increments a
/// counter when entering a block whose send method is an RSpec example method (it,
/// specify, example, scenario, etc. including f/x prefixed variants), and decrements
/// when leaving. Void expects are only flagged when `in_example > 0`.
///
/// Previous fix for 52 FPs from asciidoctor-pdf: parenthesized expect calls like
/// `(expect res.exitstatus).to be 0` create a Prism AST where the expect is inside
/// a ParenthesesNode. The visitor first collects chained expect calls (receivers of
/// `.to`/`.not_to`/`.to_not`, looking through ParenthesesNode), then flags
/// statement-level expect calls not in the chained set.
pub struct VoidExpect;

/// Matcher methods that chain on expect
const MATCHER_METHODS: &[&[u8]] = &[b"to", b"not_to", b"to_not"];

/// RSpec example method names that define example blocks
const EXAMPLE_METHODS: &[&[u8]] = &[
    b"it",
    b"specify",
    b"example",
    b"scenario",
    b"fit",
    b"fspecify",
    b"fexample",
    b"fscenario",
    b"xit",
    b"xspecify",
    b"xexample",
    b"xscenario",
];

impl Cop for VoidExpect {
    fn name(&self) -> &'static str {
        "RSpec/VoidExpect"
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
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = VoidExpectVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            chained_expect_offsets: Vec::new(),
            in_example: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct VoidExpectVisitor<'a> {
    cop: &'a VoidExpect,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Start offsets of expect calls that are receivers of .to/.not_to/.to_not
    chained_expect_offsets: Vec<usize>,
    /// Depth counter for being inside an RSpec example block (it, specify, etc.)
    in_example: usize,
}

/// If the node is a receiverless `expect` call (directly or wrapped in parentheses),
/// return its start offset.
fn extract_expect_offset(node: &ruby_prism::Node<'_>) -> Option<usize> {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"expect" && call.receiver().is_none() {
            return Some(call.location().start_offset());
        }
    }
    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                let body_nodes: Vec<_> = stmts.body().iter().collect();
                if body_nodes.len() == 1 {
                    if let Some(call) = body_nodes[0].as_call_node() {
                        if call.name().as_slice() == b"expect" && call.receiver().is_none() {
                            return Some(call.location().start_offset());
                        }
                    }
                }
            }
        }
    }
    None
}

impl Visit<'_> for VoidExpectVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        // Record chained expects: .to/.not_to/.to_not whose receiver is an expect call.
        let name = node.name();
        if MATCHER_METHODS.iter().any(|m| name.as_slice() == *m) {
            if let Some(receiver) = node.receiver() {
                if let Some(offset) = extract_expect_offset(&receiver) {
                    self.chained_expect_offsets.push(offset);
                }
            }
        }

        // Check if this call has a block and is an example method
        let is_example = node.block().is_some()
            && node.receiver().is_none()
            && EXAMPLE_METHODS.iter().any(|m| name.as_slice() == *m);

        if is_example {
            self.in_example += 1;
        }

        ruby_prism::visit_call_node(self, node);

        if is_example {
            self.in_example -= 1;
        }
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'_>) {
        // Only flag void expects when inside an example block
        if self.in_example > 0 {
            for stmt in node.body().iter() {
                // Direct expect call as a statement
                if let Some(call) = stmt.as_call_node() {
                    if call.name().as_slice() == b"expect" && call.receiver().is_none() {
                        let offset = call.location().start_offset();
                        if !self.chained_expect_offsets.contains(&offset) {
                            let (line, column) = self.source.offset_to_line_col(offset);
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.".to_string(),
                            ));
                        }
                    }
                }
                // Parenthesized expect as a statement: (expect ...)
                if let Some(parens) = stmt.as_parentheses_node() {
                    if let Some(body) = parens.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            let body_nodes: Vec<_> = stmts.body().iter().collect();
                            if body_nodes.len() == 1 {
                                if let Some(call) = body_nodes[0].as_call_node() {
                                    if call.name().as_slice() == b"expect"
                                        && call.receiver().is_none()
                                    {
                                        let offset = call.location().start_offset();
                                        if !self.chained_expect_offsets.contains(&offset) {
                                            let loc = parens.location();
                                            let (line, column) =
                                                self.source.offset_to_line_col(loc.start_offset());
                                            self.diagnostics.push(self.cop.diagnostic(
                                                self.source,
                                                line,
                                                column,
                                                "Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.".to_string(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Continue visiting child nodes
        ruby_prism::visit_statements_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(VoidExpect, "cops/rspec/void_expect");
}
