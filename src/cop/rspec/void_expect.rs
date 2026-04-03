use crate::cop::shared::method_dispatch_predicates;
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// RSpec/VoidExpect: flags `expect(...)` or `expect { ... }` calls that are not
/// chained with `.to`, `.not_to`, or `.to_not`.
///
/// Investigation (0 FP, 1 FN -> 0 FP, 0 FN):
///
/// Root cause of the remaining FN: RuboCop's Parser-based `void?` check uses AST
/// structural equality, not identity, for `parent.body == expect`. In
/// `expect { expect }.to raise_error`, the inner bare `expect` inside the block is
/// structurally identical to the outer block-form `expect` send. That makes
/// RuboCop flag BOTH the outer `expect {` send and the inner bare `expect`.
/// nitrocop was treating the outer block-form `expect` as non-void as soon as it
/// saw the chained `.to`, so it missed the extra offense on the outer send.
///
/// Fix: when a matcher method chains off a direct block-form `expect`, compare the
/// block body's sole statement against the outer `expect` send shape (receiverless
/// `expect` with the same arguments). If they match, flag the outer `expect`
/// instead of treating it as a valid chained expectation.
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
///
/// Investigation (12 FP, 0 FN -> 0 FP, 0 FN):
///
/// Root cause of 12 FPs: Explicit `begin..end` blocks map to `kwbegin` in Parser AST, NOT
/// `begin`. RuboCop's `void?` checks `parent.begin_type?` which is false for `kwbegin`. So
/// multi-statement expects inside explicit `begin..end` (without rescue/ensure) are NOT void.
/// nitrocop was flagging multi-statement StatementsNodes inside BeginNode as void, but should
/// only flag when the BeginNode has rescue/ensure (which creates an implicit `begin` wrapper
/// in Parser AST, where `begin_type?` returns true).
///
/// Fix: Added `visit_begin_node` with `pending_begin_body` counter. Only pure `begin..end`
/// blocks (no rescue/ensure/else) suppress multi-statement void detection. BeginNodes with
/// rescue/ensure still correctly flag multi-statement bodies.
///
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=12, FN=0.
///
/// FP=12: Fixed. Root cause: `it ..., &(proc do end)` uses BlockArgumentNode in
/// `call.block()`, not BlockNode. `is_example` was true for these calls, causing
/// expects inside the proc block to be flagged as void. In RuboCop,
/// `inside_example?` uses `node.each_ancestor(:block)` which only matches actual
/// block nodes, not block_pass. Fix: require `block.as_block_node().is_some()` in
/// the `is_example` check.
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
            pending_begin_body: 0,
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
    /// Counter for explicit begin..end body depth. Incremented when entering a BeginNode,
    /// decremented when the first StatementsNode inside it is visited.
    /// In Parser AST, explicit begin..end creates `kwbegin` (NOT `begin`), and
    /// `void?` checks `parent.begin_type?` which is false for `kwbegin`. So
    /// multi-statement expects inside explicit begin..end are NOT void.
    pending_begin_body: usize,
}

fn extract_direct_expect_call<'pr>(
    node: &ruby_prism::Node<'pr>,
) -> Option<ruby_prism::CallNode<'pr>> {
    let call = node.as_call_node()?;
    if method_dispatch_predicates::is_command(&call, b"expect") {
        Some(call)
    } else {
        None
    }
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
            if method_dispatch_predicates::is_command(&call, b"expect") {
                return Some(call.location().start_offset());
            }
        }
    }
    None
}

fn call_args_match_by_source(
    left: &ruby_prism::CallNode<'_>,
    right: &ruby_prism::CallNode<'_>,
) -> bool {
    let left_args: Vec<_> = left
        .arguments()
        .map(|args| args.arguments().iter().collect())
        .unwrap_or_default();
    let right_args: Vec<_> = right
        .arguments()
        .map(|args| args.arguments().iter().collect())
        .unwrap_or_default();

    left_args.len() == right_args.len()
        && left_args
            .iter()
            .zip(right_args.iter())
            .all(|(left, right)| left.location().as_slice() == right.location().as_slice())
}

/// RuboCop's Parser AST checks `parent.body == expect` using structural equality.
/// For `expect { expect }.to raise_error`, the outer block-form `expect` send is
/// structurally equal to the block body's sole bare `expect`, so RuboCop flags the
/// outer `expect` even though `.to` is chained on the full expression.
fn block_body_matches_expect_send(call: &ruby_prism::CallNode<'_>) -> bool {
    let block = match call.block().and_then(|block| block.as_block_node()) {
        Some(block) => block,
        None => return false,
    };
    let body = match block.body().and_then(|body| body.as_statements_node()) {
        Some(body) => body,
        None => return false,
    };
    let body_nodes: Vec<_> = body.body().iter().collect();
    if body_nodes.len() != 1 {
        return false;
    }
    let inner = match extract_direct_expect_call(&body_nodes[0]) {
        Some(inner) => inner,
        None => return false,
    };

    inner.block().is_none()
        && inner.receiver().is_none()
        && call.receiver().is_none()
        && inner.name().as_slice() == call.name().as_slice()
        && call_args_match_by_source(call, &inner)
}

impl VoidExpectVisitor<'_> {
    fn add_offense_at_offset(&mut self, offset: usize) {
        let (line, column) = self.source.offset_to_line_col(offset);
        self.diagnostics.push(
            self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it."
                    .to_string(),
            ),
        );
    }

    /// Check if a statement is a void expect call and flag it if so.
    fn check_void_expect_stmt(&mut self, stmt: &ruby_prism::Node<'_>) {
        // Direct expect call as a statement
        if let Some(call) = stmt.as_call_node() {
            if method_dispatch_predicates::is_command(&call, b"expect") {
                let offset = call.location().start_offset();
                if !self.chained_expect_offsets.contains(&offset) {
                    self.add_offense_at_offset(offset);
                }
            }
        }
        // Parenthesized expect as a statement: (expect ...)
        // Always void per RuboCop (parens create begin parent)
        if let Some(offset) = extract_paren_expect_offset(stmt) {
            if !self.chained_expect_offsets.contains(&offset) {
                self.add_offense_at_offset(offset);
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
                if let Some(expect_call) = extract_direct_expect_call(&receiver) {
                    let offset = expect_call.location().start_offset();
                    if self.in_example > 0 && block_body_matches_expect_send(&expect_call) {
                        self.add_offense_at_offset(offset);
                        self.chained_expect_offsets.push(offset);
                    } else {
                        self.chained_expect_offsets.push(offset);
                    }
                }
                // Parenthesized expects like `(expect x).to be 1` are void per RuboCop:
                // parens create a begin node parent for the expect send, and begin_type?
                // makes void? return true regardless of the outer .to chain.
                if self.in_example > 0 {
                    if let Some(offset) = extract_paren_expect_offset(&receiver) {
                        self.add_offense_at_offset(offset);
                        // Mark as handled so visit_statements_node doesn't flag it again
                        // when visiting the StatementsNode inside the ParenthesesNode.
                        self.chained_expect_offsets.push(offset);
                    }
                }
            }
        }

        // Check if this call has a block and is an example method.
        // Require the block to be an actual BlockNode (not BlockArgumentNode).
        // `it '...', &(proc do end)` has a BlockArgumentNode in call.block(),
        // which RuboCop does NOT treat as an example block (it uses `each_ancestor(:block)`
        // which only matches actual block nodes, not block_pass).
        let is_example = node.block().is_some_and(|b| b.as_block_node().is_some())
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

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'_>) {
        // Explicit begin..end blocks map to `kwbegin` in Parser AST.
        // In RuboCop, void? checks `parent.begin_type?` which is false for kwbegin.
        // So multi-statement expects inside explicit begin..end are NOT void.
        //
        // However, when a begin..end (or block body) has rescue/ensure, Parser wraps
        // the multi-statement body before rescue/ensure in an implicit `begin` node.
        // So `begin; a; b; rescue; end` -> the parent of `b` IS `begin` (void).
        // Only suppress when the BeginNode is a pure begin..end (no rescue/ensure).
        let is_pure_begin = node.rescue_clause().is_none()
            && node.ensure_clause().is_none()
            && node.else_clause().is_none();
        if is_pure_begin {
            self.pending_begin_body += 1;
        }
        ruby_prism::visit_begin_node(self, node);
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'_>) {
        // Track whether this StatementsNode is a block/lambda body
        let is_block_body = if self.pending_block_body > 0 {
            self.pending_block_body -= 1;
            true
        } else {
            false
        };

        // Track whether this StatementsNode is an explicit begin..end body
        let is_begin_body = if self.pending_begin_body > 0 {
            self.pending_begin_body -= 1;
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
            //   BUT: explicit begin..end creates kwbegin (NOT begin) in Parser,
            //   so multi-statement begin..end is NOT void.
            // - parent.block_type? && parent.body == expect -> true (sole statement in block)
            //   Handled by visit_block_node/visit_lambda_node above
            // - Single statement in an if/case/etc is NOT void (parent is if/case type)
            //   So we skip single-statement StatementsNodes that aren't block bodies.
            //
            // For block bodies with a single statement, visit_block_node already handled it.
            // For block bodies with multiple statements, multi_statement is true.
            // For non-block contexts (if/case/etc), only multi_statement triggers void.
            // For explicit begin..end bodies, skip (kwbegin is NOT begin_type? in Parser).
            if multi_statement && !is_begin_body {
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

    #[test]
    fn chained_block_expect_with_nested_bare_expect_is_void_twice() {
        use crate::testutil::run_cop_full;
        let cop = VoidExpect;

        let d = run_cop_full(
            &cop,
            b"it 'test' do\n  expect {\n    expect\n  }.to raise_error(ArgumentError)\nend\n",
        );
        assert_eq!(
            d.len(),
            2,
            "expected outer block-form expect and inner bare expect to both be void: {d:?}"
        );
    }

    #[test]
    fn explicit_begin_end_not_void() {
        // In Parser AST, explicit begin..end creates kwbegin (NOT begin).
        // void? checks parent.begin_type? which is false for kwbegin.
        // Multi-statement begin..end should NOT be flagged.
        use crate::testutil::run_cop_full;
        let cop = VoidExpect;

        let d = run_cop_full(
            &cop,
            b"it 'test' do\n  begin\n    setup\n    expect(result)\n  end\nend\n",
        );
        assert!(
            d.is_empty(),
            "multi-stmt explicit begin..end should not be void: {d:?}"
        );

        // But begin..end WITH rescue DOES flag (rescue body is begin_type? in Parser)
        let d = run_cop_full(
            &cop,
            b"it 'test' do\n  begin\n    setup\n    expect(result)\n  rescue\n    nil\n  end\nend\n",
        );
        assert_eq!(d.len(), 1, "multi-stmt begin..rescue should be void: {d:?}");
    }
}
