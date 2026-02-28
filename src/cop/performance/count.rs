use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct Count;

impl Cop for Count {
    fn name(&self) -> &'static str {
        "Performance/Count"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        let mut visitor = CountVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_single_stmt_block_body: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct CountVisitor<'a, 'src> {
    cop: &'a Count,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when we're visiting the sole statement of a block body.
    /// RuboCop skips `select{}.count` when it's the only expression in a
    /// block body (parent is block_type? in parser-gem terms).
    in_single_stmt_block_body: bool,
}

impl<'pr> Visit<'pr> for CountVisitor<'_, '_> {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Check if the block has exactly one statement in its body.
        let is_single = is_single_statement_body(node.body());
        if is_single {
            let prev = self.in_single_stmt_block_body;
            self.in_single_stmt_block_body = true;
            ruby_prism::visit_block_node(self, node);
            self.in_single_stmt_block_body = prev;
        } else {
            ruby_prism::visit_block_node(self, node);
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambdas are block-like in parser gem
        let is_single = is_single_statement_body(node.body());
        if is_single {
            let prev = self.in_single_stmt_block_body;
            self.in_single_stmt_block_body = true;
            ruby_prism::visit_lambda_node(self, node);
            self.in_single_stmt_block_body = prev;
        } else {
            ruby_prism::visit_lambda_node(self, node);
        }
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_call(node);
        // When visiting children of a call node, we're no longer at the
        // top level of a single-statement block body. The block's sole
        // statement is *this* call; its sub-expressions are deeper.
        let prev = self.in_single_stmt_block_body;
        self.in_single_stmt_block_body = false;
        ruby_prism::visit_call_node(self, node);
        self.in_single_stmt_block_body = prev;
    }
}

impl CountVisitor<'_, '_> {
    fn check_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Outer method must be count/size/length
        let outer = call.name().as_slice();
        let outer_name = match outer {
            b"count" => "count",
            b"size" => "size",
            b"length" => "length",
            _ => return,
        };

        // Must have a receiver
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver must be a CallNode (the inner select/reject/filter/find_all)
        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let inner = inner_call.name().as_slice();
        let inner_name = match inner {
            b"select" => "select",
            b"reject" => "reject",
            b"filter" => "filter",
            b"find_all" => "find_all",
            _ => return,
        };

        // The inner call must have a block (normal block or block_pass like &:symbol)
        let inner_block = match inner_call.block() {
            Some(b) => b,
            None => return,
        };

        // If the block is a regular block (not block_pass), it must have a body.
        // RuboCop: `node.receiver.call_type? || node.receiver.body`
        // block_pass → call_type? is true (receiver is call node in parser-gem)
        // regular block → check body is present (non-empty block)
        if let Some(block_node) = inner_block.as_block_node() {
            if block_node.body().is_none() {
                return;
            }

            // RuboCop's Parser gem has separate `block` and `numblock` node types.
            // `numblock` (used for _1/_2 numbered params and Ruby 3.4 `it`) returns
            // false for `block_type?`, causing RuboCop to skip these chains.
            // Match that behavior: skip when the block uses numbered or it params.
            if let Some(params) = block_node.parameters() {
                if params.as_numbered_parameters_node().is_some()
                    || params.as_it_parameters_node().is_some()
                {
                    return;
                }
            }
        }

        // Skip if the outer call (count/size/length) itself has a block:
        // e.g. `select { |e| e.odd? }.count { |e| e > 2 }` is allowed
        if call.block().is_some() {
            return;
        }

        // Skip if this call is the sole body of a block.
        // RuboCop: `return false if node.parent&.block_type?`
        // In parser-gem, when a block has one statement, the statement's
        // parent IS the block node. With multiple statements they're
        // wrapped in a `begin` node, so the parent is `begin`, not block.
        if self.in_single_stmt_block_body {
            return;
        }

        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!("Use `count` instead of `{inner_name}...{outer_name}`."),
        ));
    }
}

/// Check if a block/lambda body has exactly one statement.
fn is_single_statement_body(body: Option<ruby_prism::Node<'_>>) -> bool {
    let body = match body {
        Some(b) => b,
        None => return false,
    };
    match body.as_statements_node() {
        Some(stmts) => stmts.body().len() == 1,
        // Body is a single non-statements node
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Count, "cops/performance/count");
}
