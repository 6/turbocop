use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// RSpec/VoidExpect: flags `expect(...)` or `expect { ... }` calls that are not
/// chained with `.to`, `.not_to`, or `.to_not`.
///
/// Investigation (0 FP, 6804 FN -> 0 FP, 0 FN):
///
/// Root cause of 6,804 FNs: RuboCop's `void?` check uses the Parser AST's parent
/// relationship. For parenthesized expects like `(expect x).to be 1`, the Parser AST
/// wraps `expect x` in a `begin` node (from the parentheses), and `begin_type?` makes
/// `void?` return true — EVEN when `.to` is chained on the outer parens. This means
/// RuboCop intentionally flags ALL parenthesized `expect` calls as void, regardless of
/// whether `.to`/`.not_to`/`.to_not` is chained on the parenthesized expression.
///
/// The previous fix incorrectly excluded parenthesized chained expects from the void
/// check (via `extract_expect_offset` looking through ParenthesesNode), causing
/// nitrocop to NOT flag `(expect x).to be 1` — but RuboCop DOES flag it.
///
/// Fix: Changed `extract_expect_offset` to only match direct `expect` CallNodes (not
/// parenthesized). Added a new check in `visit_call_node` to detect `.to`/`.not_to`/
/// `.to_not` calls whose receiver is a ParenthesesNode containing an `expect` call,
/// and flag those expects when inside an example. Also added missing example methods:
/// `its`, `focus`, `skip`, `pending`.
///
/// Investigation (12 FP, 0 FN -> 0 FP, 0 FN):
///
/// Root cause of 12 FPs: RuboCop's `void?` method checks the Parser AST parent type:
/// `parent.begin_type?` (multi-statement body) or `parent.block_type? && parent.body == expect`
/// (sole statement in block). A sole `expect` inside a conditional (`if`, `unless`, `case/when`,
/// modifier `if`/`unless`) has an `if_type?`/`case_type?` parent — NOT `begin_type?` — so
/// RuboCop does NOT flag it as void. nitrocop was using `visit_statements_node` to flag ALL
/// standalone expects in any StatementsNode, regardless of what contained the StatementsNode.
///
/// Fix: Split void-expect detection into two cases matching RuboCop's logic:
///
/// 1. `visit_block_node`/`visit_lambda_node` handle sole-statement block bodies (block_type?)
/// 2. `visit_statements_node` only flags expects in multi-statement contexts (begin_type?)
///
/// Single-statement conditionals/loops are no longer flagged since they don't match either case.
pub struct VoidExpect;

/// Matcher methods that chain on expect
const MATCHER_METHODS: &[&[u8]] = &[b"to", b"not_to", b"to_not"];

/// RSpec example method names that define example blocks.
/// Must match RuboCop's `Examples.all` from the Language config:
/// Regular: it, specify, example, scenario, its
/// Focused: fit, fspecify, fexample, fscenario, focus
/// Skipped: xit, xspecify, xexample, xscenario, skip
/// Pending: pending
const EXAMPLE_METHODS: &[&[u8]] = &[
    b"it",
    b"specify",
    b"example",
    b"scenario",
    b"its",
    b"fit",
    b"fspecify",
    b"fexample",
    b"fscenario",
    b"focus",
    b"xit",
    b"xspecify",
    b"xexample",
    b"xscenario",
    b"skip",
    b"pending",
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
            pending_block_body: 0,
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
    /// Counter for block body depth. Incremented when entering a BlockNode/LambdaNode,
    /// decremented when the first StatementsNode inside it is visited.
    /// Used to distinguish block body StatementsNode from conditional branch StatementsNode.
    pending_block_body: usize,
}

/// If the node is a DIRECT receiverless `expect` call (NOT wrapped in parentheses),
/// return its start offset. Parenthesized expects like `(expect x)` are excluded
/// because RuboCop treats them as void even when `.to` is chained.
fn extract_direct_expect_offset(node: &ruby_prism::Node<'_>) -> Option<usize> {
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"expect" && call.receiver().is_none() {
            return Some(call.location().start_offset());
        }
    }
    None
}

/// If the node is a ParenthesesNode containing a single receiverless `expect` call,
/// return the expect call's start offset.
fn extract_paren_expect_offset(node: &ruby_prism::Node<'_>) -> Option<usize> {
    let parens = node.as_parentheses_node()?;
    let body = parens.body()?;
    let stmts = body.as_statements_node()?;
    let body_nodes: Vec<_> = stmts.body().iter().collect();
    if body_nodes.len() == 1 {
        if let Some(call) = body_nodes[0].as_call_node() {
            if call.name().as_slice() == b"expect" && call.receiver().is_none() {
                return Some(call.location().start_offset());
            }
        }
    }
    None
}

impl VoidExpectVisitor<'_> {
    /// Check if a statement is a void expect call and flag it if so.
    fn check_void_expect_stmt(&mut self, stmt: &ruby_prism::Node<'_>) {
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
        // Always void per RuboCop (parens create begin parent)
        if let Some(offset) = extract_paren_expect_offset(stmt) {
            if !self.chained_expect_offsets.contains(&offset) {
                let (line, column) = self.source.offset_to_line_col(offset);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.".to_string(),
                ));
                // Mark as handled so inner StatementsNode visit doesn't double-flag
                self.chained_expect_offsets.push(offset);
            }
        }
    }
}

impl Visit<'_> for VoidExpectVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        let name = node.name();

        // For .to/.not_to/.to_not calls, check the receiver:
        // 1. Direct expect receiver -> record as chained (non-void)
        // 2. Parenthesized expect receiver -> flag as void (RuboCop's begin_type? logic)
        if MATCHER_METHODS.iter().any(|m| name.as_slice() == *m) {
            if let Some(receiver) = node.receiver() {
                if let Some(offset) = extract_direct_expect_offset(&receiver) {
                    self.chained_expect_offsets.push(offset);
                }
                // Parenthesized expects like `(expect x).to be 1` are void per RuboCop:
                // parens create a begin node parent for the expect send, and begin_type?
                // makes void? return true regardless of the outer .to chain.
                if self.in_example > 0 {
                    if let Some(offset) = extract_paren_expect_offset(&receiver) {
                        let (line, column) = self.source.offset_to_line_col(offset);
                        self.diagnostics.push(self.cop.diagnostic(
                            self.source,
                            line,
                            column,
                            "Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it.".to_string(),
                        ));
                        // Mark as handled so visit_statements_node doesn't flag it again
                        // when visiting the StatementsNode inside the ParenthesesNode.
                        self.chained_expect_offsets.push(offset);
                    }
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

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'_>) {
        // For single-statement block bodies, check if the sole statement is a void expect.
        // In RuboCop: parent.block_type? && parent.body == expect -> void.
        // Multi-statement block bodies are handled by visit_statements_node (begin_type? logic).
        if self.in_example > 0 {
            if let Some(body) = node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    if body_nodes.len() == 1 {
                        self.check_void_expect_stmt(&body_nodes[0]);
                    }
                }
            }
        }
        // Mark that the next StatementsNode is a block body (for multi-statement handling)
        self.pending_block_body += 1;
        ruby_prism::visit_block_node(self, node);
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'_>) {
        if self.in_example > 0 {
            if let Some(body) = node.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().iter().collect();
                    if body_nodes.len() == 1 {
                        self.check_void_expect_stmt(&body_nodes[0]);
                    }
                }
            }
        }
        self.pending_block_body += 1;
        ruby_prism::visit_lambda_node(self, node);
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'_>) {
        // Track whether this StatementsNode is a block/lambda body
        let is_block_body = if self.pending_block_body > 0 {
            self.pending_block_body -= 1;
            true
        } else {
            false
        };

        // Only flag void expects when inside an example block
        if self.in_example > 0 {
            let stmts: Vec<_> = node.body().iter().collect();
            let multi_statement = stmts.len() > 1;

            // RuboCop's void? logic:
            // - parent.begin_type? -> true (multi-statement body in Parser AST)
            //   Maps to: multiple statements in a StatementsNode
            // - parent.block_type? && parent.body == expect -> true (sole statement in block)
            //   Handled by visit_block_node/visit_lambda_node above
            // - Single statement in an if/case/etc is NOT void (parent is if/case type)
            //   So we skip single-statement StatementsNodes that aren't block bodies.
            //
            // For block bodies with a single statement, visit_block_node already handled it.
            // For block bodies with multiple statements, multi_statement is true.
            // For non-block contexts (if/case/etc), only multi_statement triggers void.
            if multi_statement {
                for stmt in &stmts {
                    self.check_void_expect_stmt(stmt);
                }
            } else if is_block_body {
                // Single-statement block body: already handled in visit_block_node,
                // so don't flag again here. Just consume the pending flag.
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
