use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Rails/EagerEvaluationLogMessage
///
/// Flags `Rails.logger.debug "#{interpolated}"` calls that pass an eager-evaluated
/// interpolated string instead of a lazy block. Matches vendor pattern:
/// `(send (send (const {cbase nil?} :Rails) :logger) :debug (dstr ...))`
///
/// ## Investigation (2026-03-16)
///
/// **Root cause of FN=12**: The `sole_block_stmt` flag was not being reset when
/// entering a nested block with multiple statements. When an outer block has a single
/// statement (e.g., `items.each do |item| Post.transaction do ... end end`), the flag
/// is set to `true`. If the inner block (`Post.transaction do`) has multiple statements,
/// the debug call inside it IS an offense — but the inherited `sole_block_stmt=true`
/// caused it to be skipped. Fix: reset `sole_block_stmt=false` when descending into a
/// multi-statement (or no-body) block.
///
/// **Confirmed patterns** seen in corpus FNs (discourse/discourse, theforeman/foreman, etc.):
/// ```ruby
/// items.each do |item|
///   Post.transaction do
///     Rails.logger.debug "Processing #{item.name}"  # was incorrectly skipped
///     do_something(item)
///   end
/// end
/// ```
///
/// ## Investigation (2026-03-18)
///
/// **Root cause of FN=4**: The `sole_block_stmt` flag was propagating through
/// compound statement containers (if/unless/case/while/until/rescue/begin). When an
/// outer block has a single statement that is an `if` node, `sole_block_stmt = true`
/// was set. The default visitor then descended into the if body without resetting the
/// flag, so debug calls inside the conditional were incorrectly skipped.
///
/// RuboCop's check `return if node.parent&.block_type?` only returns when the debug
/// call's DIRECT parent is a block node, so conditionals nested inside single-stmt
/// blocks do NOT skip (the debug call's parent is the if body, not a block).
///
/// Fix: add visitor overrides for if/unless/case/while/until/rescue/begin that reset
/// `sole_block_stmt = false` before descending.
pub struct EagerEvaluationLogMessage;

impl Cop for EagerEvaluationLogMessage {
    fn name(&self) -> &'static str {
        "Rails/EagerEvaluationLogMessage"
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
        let mut visitor = EagerEvalVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            sole_block_stmt: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct EagerEvalVisitor<'a> {
    cop: &'a EagerEvaluationLogMessage,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when visiting the sole statement inside a block body.
    /// Matches RuboCop's `return if node.parent&.block_type?` — in Parser AST,
    /// a block with a single statement has the statement as a direct child of the
    /// block node (no `begin` wrapper), so `parent.block_type?` is true.
    ///
    /// IMPORTANT: This flag must be reset to false when entering a nested block
    /// that has multiple statements. Otherwise, debug calls inside a multi-statement
    /// inner block would be skipped because the flag was set true by the outer
    /// single-statement block. Example: `items.each { Post.transaction do <debug>; <other>; end }`
    /// — the outer each block has 1 stmt so sole_block_stmt=true, but the inner
    /// transaction block has 2 stmts so the debug inside it IS an offense.
    sole_block_stmt: bool,
}

impl<'pr> Visit<'pr> for EagerEvalVisitor<'_> {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // If the block body has exactly 1 statement, set the flag while visiting it.
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let count = stmts.body().iter().count();
                if count == 1 {
                    let was = self.sole_block_stmt;
                    self.sole_block_stmt = true;
                    self.visit(&body);
                    self.sole_block_stmt = was;
                    return;
                }
            }
        }
        // Multiple statements (or no body): reset flag so nested debug calls ARE checked.
        // This is necessary because an outer single-statement block sets sole_block_stmt=true,
        // but a nested multi-statement block must not inherit that flag — its debug calls
        // are not sole statements and must be flagged.
        let was = self.sole_block_stmt;
        self.sole_block_stmt = false;
        ruby_prism::visit_block_node(self, node);
        self.sole_block_stmt = was;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_debug_call(node);
        ruby_prism::visit_call_node(self, node);
    }

    // When descending into compound statement containers (if/unless/case/while/until/rescue),
    // reset sole_block_stmt to false. This prevents the "outer single-stmt block" flag from
    // propagating into nested conditional bodies, where the debug call's parent is NOT a
    // block_type (matching RuboCop's `return if node.parent&.block_type?` semantics).
    //
    // Example that was incorrectly skipped:
    //   items.each do |item|   # sole_block_stmt = true (1 stmt = the if node)
    //     if item.valid?
    //       Rails.logger.debug "Processing #{item.name}"  # should fire
    //     end
    //   end
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let was = self.sole_block_stmt;
        self.sole_block_stmt = false;
        ruby_prism::visit_if_node(self, node);
        self.sole_block_stmt = was;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let was = self.sole_block_stmt;
        self.sole_block_stmt = false;
        ruby_prism::visit_unless_node(self, node);
        self.sole_block_stmt = was;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let was = self.sole_block_stmt;
        self.sole_block_stmt = false;
        ruby_prism::visit_case_node(self, node);
        self.sole_block_stmt = was;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        let was = self.sole_block_stmt;
        self.sole_block_stmt = false;
        ruby_prism::visit_while_node(self, node);
        self.sole_block_stmt = was;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let was = self.sole_block_stmt;
        self.sole_block_stmt = false;
        ruby_prism::visit_until_node(self, node);
        self.sole_block_stmt = was;
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        let was = self.sole_block_stmt;
        self.sole_block_stmt = false;
        ruby_prism::visit_rescue_node(self, node);
        self.sole_block_stmt = was;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let was = self.sole_block_stmt;
        self.sole_block_stmt = false;
        ruby_prism::visit_begin_node(self, node);
        self.sole_block_stmt = was;
    }
}

impl EagerEvalVisitor<'_> {
    fn check_debug_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        if call.name().as_slice() != b"debug" {
            return;
        }

        // If already using a block, skip
        if call.block().is_some() {
            return;
        }

        // RuboCop: `return if node.parent&.block_type?` — skip when the debug call
        // is the sole statement in a block body.
        if self.sole_block_stmt {
            return;
        }

        // RuboCop's pattern matches `send` (not `csend`), so safe navigation
        // `Rails.logger&.debug(...)` is excluded.
        if let Some(op) = call.call_operator_loc() {
            if self.source.as_bytes()[op.start_offset()..op.end_offset()] == *b"&." {
                return;
            }
        }

        // Receiver must be Rails.logger (a 2-method chain)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if inner_call.name().as_slice() != b"logger" {
            return;
        }

        // Inner receiver must be `Rails` constant
        let inner_recv = match inner_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_rails = if let Some(cr) = inner_recv.as_constant_read_node() {
            cr.name().as_slice() == b"Rails"
        } else if let Some(cp) = inner_recv.as_constant_path_node() {
            // ::Rails
            cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"Rails")
        } else {
            false
        };

        if !is_rails {
            return;
        }

        // First argument must be an interpolated string (dstr)
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        if arg_list[0].as_interpolated_string_node().is_none() {
            return;
        }

        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Pass a block to `Rails.logger.debug`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        EagerEvaluationLogMessage,
        "cops/rails/eager_evaluation_log_message"
    );
}
