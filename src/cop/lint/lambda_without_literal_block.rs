use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks uses of `lambda` without a literal block.
///
/// ## Investigation findings (2026-03-14)
///
/// Root causes of corpus FP/FN (FP=6, FN=3):
///
/// 1. **FP (jruby 3, natalie 3):** nitrocop flagged `lambda(&pr)` inside block
///    bodies like `-> { lambda(&proc{}) }` and `suppress_warning { lambda(&body) }`.
///    RuboCop skips these because in parser gem's AST, the `send` node's parent is
///    a `block` node (`node.parent&.block_type?`). In Prism, the CallNode is inside
///    a StatementsNode inside a BlockNode/LambdaNode. Fix: use `check_source` with
///    a visitor that tracks block body context.
///
/// 2. **FN (awspec 2):** `describe lambda('my-func') do...end` — nitrocop only
///    checked for `&` block arguments, but RuboCop flags ANY `lambda` call with
///    arguments that doesn't have a literal block (the check is `!node.first_argument`).
///    Fix: flag when lambda has any arguments OR a non-symbol block pass, and no
///    literal block.
///
/// 3. **FN (slim 1):** `@parent.lambda(name, &block)` — nitrocop required no
///    receiver, but RuboCop's `RESTRICT_ON_SEND` only filters by method name and
///    doesn't check receiver. Fix: remove receiver check.
///
/// ## Investigation findings (2026-03-28)
///
/// FN=1: `reduce(&lambda(&method(:longest_words)))` in ruby-spark. The manual
/// `visit_call_node` walk skipped `BlockArgumentNode` children entirely, so
/// nested `lambda` calls inside another call's `&...` block pass were never
/// visited. Fix: visit `BlockArgumentNode` expressions, but keep them
/// non-transparent for `parent_is_block_body` tracking so only direct block-body
/// children stay exempt, matching RuboCop's `node.parent&.block_type?`.
///
/// ## Investigation findings (2026-04-03)
///
/// FN=1: `lambda(&b).call` in matplotlib.rb. We correctly exempted a `lambda`
/// call that is itself a direct child of a block body, but incorrectly leaked
/// that exemption into the child receiver/argument/block-pass walk of the outer
/// `.call`. In parser gem, the inner `lambda` send's immediate parent is the
/// outer send, not the block, so RuboCop still flags it. Fix: clear
/// `parent_is_block_body` before visiting a call's receiver, arguments, and
/// non-literal block child; only the call node itself and literal block bodies
/// retain the direct block-body context.
pub struct LambdaWithoutLiteralBlock;

impl Cop for LambdaWithoutLiteralBlock {
    fn name(&self) -> &'static str {
        "Lint/LambdaWithoutLiteralBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let mut walker = LambdaWalker {
            source,
            cop: self,
            parent_is_block_body: false,
            saved_flags: Vec::new(),
            diagnostics,
        };
        walker.visit(&parse_result.node());
    }
}

/// Visitor that tracks whether the current position is a direct child of a
/// block body. In parser gem's AST, `node.parent&.block_type?` checks if the
/// send node's immediate parent is a block node. In Prism, block bodies are
/// wrapped in StatementsNode, so we track when we enter a StatementsNode that
/// belongs to a BlockNode or LambdaNode.
struct LambdaWalker<'a> {
    source: &'a SourceFile,
    cop: &'a LambdaWithoutLiteralBlock,
    parent_is_block_body: bool,
    /// Stack for save/restore of parent_is_block_body across branch nodes.
    saved_flags: Vec<bool>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for LambdaWalker<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"lambda" {
            self.check_lambda_call(node);
        }

        // Visit children manually. The call's block body (if BlockNode) creates
        // a new direct block-body context; the receiver, arguments, and a
        // non-literal `&...` child do not.
        let saved = self.parent_is_block_body;

        if let Some(recv) = node.receiver() {
            self.parent_is_block_body = false;
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            self.parent_is_block_body = false;
            self.visit(&args.as_node());
        }
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                // Entering a literal block body — direct children are in block
                // body context.
                self.parent_is_block_body = true;
                if let Some(body) = block_node.body() {
                    self.visit(&body);
                }
                if let Some(params) = block_node.parameters() {
                    self.parent_is_block_body = false;
                    self.visit(&params);
                }
                self.parent_is_block_body = saved;
            } else {
                self.parent_is_block_body = false;
                self.visit(&block);
            }
        }

        self.parent_is_block_body = saved;
    }

    fn visit_block_argument_node(&mut self, node: &ruby_prism::BlockArgumentNode<'pr>) {
        if let Some(expr) = node.expression() {
            self.visit(&expr);
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambda literal `-> { ... }` — body is in block body context
        let saved = self.parent_is_block_body;
        self.parent_is_block_body = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        if let Some(params) = node.parameters() {
            self.parent_is_block_body = false;
            self.visit(&params);
        }
        self.parent_is_block_body = saved;
    }

    // For all non-transparent compound nodes (if, while, def, class, begin,
    // etc.), their children are NOT direct children of a block body. We save
    // and clear the flag on enter, and restore on leave.
    //
    // CallNode and LambdaNode handle their own save/restore above.
    // StatementsNode and ProgramNode are transparent (parser gem has no
    // equivalent wrapper — the block body IS the statement list).
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.saved_flags.push(self.parent_is_block_body);
        if node.as_call_node().is_some()
            || node.as_lambda_node().is_some()
            || node.as_statements_node().is_some()
            || node.as_program_node().is_some()
        {
            // Transparent or handled explicitly — don't change the flag
        } else {
            // Non-transparent compound node breaks the "direct child of block body"
            // relationship, matching parser gem's behavior
            self.parent_is_block_body = false;
        }
    }

    fn visit_branch_node_leave(&mut self) {
        if let Some(saved) = self.saved_flags.pop() {
            self.parent_is_block_body = saved;
        }
    }
}

impl LambdaWalker<'_> {
    fn check_lambda_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // In parser gem, RuboCop checks: node.parent&.block_type?
        // This skips lambda calls that are direct children of a block body.
        if self.parent_is_block_body {
            return;
        }

        // Check if the call has a literal block (BlockNode) — that's fine
        if let Some(block) = call.block() {
            if block.as_block_node().is_some() {
                return;
            }

            // BlockArgumentNode — check for symbol procs
            if let Some(block_arg) = block.as_block_argument_node() {
                if let Some(expr) = block_arg.expression() {
                    if expr.as_symbol_node().is_some() {
                        // lambda(&:do_something) — allowed
                        return;
                    }
                }
                // lambda(&pr) — offense
                self.add_offense(call);
                return;
            }
        }

        // No block — check if there are any arguments
        // RuboCop: `!node.first_argument` returns true when no arguments → skip
        // If there ARE arguments (string, variable, etc.), it's an offense
        let has_arguments = call
            .arguments()
            .is_some_and(|args| !args.arguments().is_empty());
        if !has_arguments {
            return;
        }

        // lambda('string'), lambda(var), etc. — no literal block
        self.add_offense(call);
    }

    fn add_offense(&mut self, call: &ruby_prism::CallNode<'_>) {
        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "lambda without a literal block is deprecated; use the proc without lambda instead."
                .to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        LambdaWithoutLiteralBlock,
        "cops/lint/lambda_without_literal_block"
    );
}
