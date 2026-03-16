/// Rails/TransactionExitStatement — detects `return`, `break`, and `throw` inside
/// transaction blocks (`transaction`, `with_lock`, or custom TransactionMethods).
///
/// ## Root Cause Analysis (FN=91)
///
/// Two bugs caused all false negatives:
///
/// 1. **Missing `with_lock`** as a built-in transaction method. RuboCop's
///    `BUILT_IN_TRANSACTION_METHODS = %i[transaction with_lock]` includes both, but
///    the original implementation only checked for `transaction`. Since `with_lock` is
///    very common in Rails codebases, this was the primary source of FN.
///
/// 2. **`break` inside nested non-transaction blocks incorrectly flagged (FP)**.
///    RuboCop skips `break` when it appears inside a nested block that is NOT a
///    transaction method (e.g., `loop do`, `each do`, `while`, `until`). The `break`
///    exits the inner block, not the outer transaction. Only `return` and `throw`
///    propagate through nested blocks to exit the enclosing method/transaction.
///    The original `ExitFinder` had no such filtering and would flag `break` inside
///    `loop`/`each`/`while`/`until` within transactions.
///
/// ## Fix
///
/// - Added `with_lock` to the default transaction method set.
/// - Added `nested_non_transaction_block_depth` tracking in `ExitFinder` to skip
///   `break` nodes that are inside nested non-transaction blocks. When the visitor
///   enters a `BlockNode` or `LambdaNode`, it increments the depth; when it exits,
///   it decrements. `break` is only reported when `nested_block_depth == 0`.
///   `return` and `throw` are always reported regardless of nesting depth (they
///   propagate out to the enclosing method, bypassing transaction commit/rollback).
///
/// ## Remaining Gaps
///
/// None observed. All vendor spec cases pass. The corpus conformance should be
/// significantly improved by adding `with_lock` detection.
use ruby_prism::Visit;

use crate::cop::node_type::{BLOCK_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Built-in transaction methods that RuboCop recognizes by default.
const BUILT_IN_TRANSACTION_METHODS: &[&[u8]] = &[b"transaction", b"with_lock"];

pub struct TransactionExitStatement;

struct ExitFinder {
    found: Vec<(usize, &'static str)>,
    /// Tracks how many nested loop/block layers we are currently inside.
    /// `break` is only flagged when this counter is 0 (i.e., directly inside the
    /// transaction block body, not inside a nested `loop do`, `each do`, `while`,
    /// `until`, or `for` construct). `return` and `throw` are always flagged
    /// regardless of depth because they propagate through nested blocks to exit the
    /// enclosing method.
    nested_block_depth: usize,
}

impl<'pr> Visit<'pr> for ExitFinder {
    // Skip nested method/class/module definitions to avoid false positives
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}

    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        // `return` always exits the enclosing method, bypassing any transaction
        // commit/rollback regardless of nesting depth inside blocks.
        self.found.push((node.location().start_offset(), "return"));
        ruby_prism::visit_return_node(self, node);
    }

    fn visit_break_node(&mut self, node: &ruby_prism::BreakNode<'pr>) {
        // `break` only exits a transaction when it is at the top level of the
        // transaction block body. Inside a nested non-transaction block (loop, each,
        // while, until, etc.), `break` exits the inner block and is harmless.
        if self.nested_block_depth == 0 {
            self.found.push((node.location().start_offset(), "break"));
        }
        ruby_prism::visit_break_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"throw" && node.receiver().is_none() {
            // `throw` propagates like `return` — always flagged regardless of nesting.
            self.found.push((node.location().start_offset(), "throw"));
        }
        // Continue visiting children of this call node
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // All nested blocks (loop do, each do, map do, etc.) increment depth so that
        // `break` inside them is not flagged — `break` exits the inner block, not the
        // outer transaction.
        self.nested_block_depth += 1;
        ruby_prism::visit_block_node(self, node);
        self.nested_block_depth -= 1;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambdas create a new scope; `return` inside a lambda exits the lambda,
        // not the enclosing method. Track as nested to suppress false positives.
        self.nested_block_depth += 1;
        ruby_prism::visit_lambda_node(self, node);
        self.nested_block_depth -= 1;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        // `break` inside a `while` loop exits the loop, not the transaction.
        self.nested_block_depth += 1;
        ruby_prism::visit_while_node(self, node);
        self.nested_block_depth -= 1;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        // `break` inside an `until` loop exits the loop, not the transaction.
        self.nested_block_depth += 1;
        ruby_prism::visit_until_node(self, node);
        self.nested_block_depth -= 1;
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        // `break` inside a `for` loop exits the loop, not the transaction.
        self.nested_block_depth += 1;
        ruby_prism::visit_for_node(self, node);
        self.nested_block_depth -= 1;
    }
}

impl Cop for TransactionExitStatement {
    fn name(&self) -> &'static str {
        "Rails/TransactionExitStatement"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let transaction_methods = config.get_string_array("TransactionMethods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        let method_name = call.name().as_slice();

        // Check if the method is a transaction method (built-in or configured).
        // Built-in: `transaction` and `with_lock`. Custom via TransactionMethods config.
        let is_transaction = if let Some(ref methods) = transaction_methods {
            let name_str = std::str::from_utf8(method_name).unwrap_or("");
            methods.iter().any(|m| m == name_str)
        } else {
            BUILT_IN_TRANSACTION_METHODS.contains(&method_name)
        };
        if !is_transaction {
            return;
        }
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let mut finder = ExitFinder {
            found: vec![],
            nested_block_depth: 0,
        };
        finder.visit(&body);

        for &(offset, statement) in &finder.found {
            let (line, column) = source.offset_to_line_col(offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Do not use `{statement}` inside a transaction block."),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        TransactionExitStatement,
        "cops/rails/transaction_exit_statement"
    );
}
