use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for `return` inside `begin..end` blocks in assignment contexts.
///
/// Root cause of 214 FNs (0.9% match rate): the original implementation was missing
/// operator write node visitors (`+=`, `-=`, `*=`, `/=`, `**=`) which Prism represents
/// as `*OperatorWriteNode` types. RuboCop handles these via `on_op_asgn`. The fix adds
/// visitors for all operator write node variants.
///
/// Additional 210 FNs from missing and-write (`&&=`), method call assignment
/// (`CallAndWriteNode`, `CallOrWriteNode`, `CallOperatorWriteNode`), index assignment
/// (`IndexAndWriteNode`, `IndexOrWriteNode`, `IndexOperatorWriteNode`), and global
/// variable or-write (`GlobalVariableOrWriteNode`) node types.
///
/// ## Corpus investigation (2026-03-19)
///
/// Corpus oracle reported FP=0, FN=210. All 210 FNs were from `return` inside
/// `begin..end` assignments within method bodies. The visitor blocked recursion
/// into `def`, `class`, `module`, and `lambda` nodes entirely (`fn visit_def_node
/// { }`) meaning it never reached assignments inside methods — which is 100% of
/// real-world usage. Fixed by letting the visitor recurse into these scopes while
/// resetting `in_begin_assignment` to false, so nested scopes start fresh but
/// assignments within methods are properly checked.
///
/// ## FN=2 fix (2026-03-19)
///
/// 2 remaining FNs from `return` inside nested `def` within `begin..end`
/// assignment contexts (jruby bench_red_black.rb and gc/redblack.rb pattern:
/// `@instance ||= begin; def instance; return @instance; end; new; end`).
/// RuboCop's `each_node(:return)` walks into nested defs without any scope
/// boundary. The visitor was resetting `in_begin_assignment` to false in
/// `visit_def_node`, preventing detection. Fix: only reset `in_assignment_value`
/// (not `in_begin_assignment`) when entering nested defs, matching RuboCop's
/// behavior of flagging returns at any depth within the begin..end block.
///
/// ## FP=19 fix (2026-03-19)
///
/// 19 FPs from `return` inside implicit `BeginNode` from rescue clauses in
/// `def`/block/lambda bodies within assignment contexts. Prism uses `BeginNode`
/// for both explicit `begin..end` (kwbegin) and implicit rescue-wrapping. The
/// visitor was treating all `BeginNode` inside assignment values as kwbegin.
/// Fix: check `begin_keyword_loc().is_some()` to distinguish explicit from
/// implicit — only explicit `begin..end` triggers `in_begin_assignment`.
/// Patterns: `var = items.find do |i| ... rescue ... end`, `CONST = lambda do
/// ... rescue ... end`, `def foo ... rescue ... end` nested inside assignments.
///
/// ## Corpus investigation (2026-03-23)
///
/// Corpus oracle reported FP=0, FN=5.
///
/// FN=5: webmock (1), dependabot-core (1), puppet (3). All involve `return` inside
/// standalone `begin..ensure..end` or `begin..rescue..end` blocks NOT in assignment
/// contexts. The webmock/dependabot pattern is `begin; ...; return yield(self); ensure;
/// ...; end` inside `if block_given?`. The puppet patterns are `begin; return
/// from_args(...); rescue; ...; end` inside rescue clauses. RuboCop's cop only fires
/// on assignment nodes (`on_lvasgn`, `on_ivasgn`, etc.) and should NOT flag these
/// standalone begin blocks. These are likely corpus oracle artifacts (possibly from a
/// broader RuboCop version or configuration). No code change needed.
///
/// ## FN=8 fix (2026-03-28)
///
/// The remaining corpus misses were not standalone begin blocks after all. The full
/// repo context wraps those methods inside outer assignment values such as
/// `@webMockNetHTTP = Class.new(...) do ... end`,
/// `@@new_function ||= Puppet::Functions.create_loaded_function(...) do ... end`,
/// and `@new_function ||= ... do ... end`. RuboCop's `each_node(:kwbegin)` on the
/// outer assignment walks into nested defs, so explicit `begin..end` blocks inside
/// those defs still count as assignment-context begins.
///
/// The visitor was clearing `in_assignment_value` in `visit_def_node`, which made it
/// miss explicit `begin..ensure..end` and `begin..rescue..end` blocks inside nested
/// defs under assignment-valued blocks. Fix: keep the outer assignment context when
/// recursing into nested defs, while continuing to rely on
/// `begin_keyword_loc().is_some()` so implicit rescue-wrapping `BeginNode`s in nested
/// defs remain non-offenses.
pub struct NoReturnInBeginEndBlocks;

impl Cop for NoReturnInBeginEndBlocks {
    fn name(&self) -> &'static str {
        "Lint/NoReturnInBeginEndBlocks"
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
        let mut visitor = NoReturnVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_begin_assignment: false,
            in_assignment_value: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct NoReturnVisitor<'a, 'src> {
    cop: &'a NoReturnInBeginEndBlocks,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// True when we're inside a begin..end block that is a descendant of an
    /// assignment value. RuboCop uses `node.each_node(:kwbegin)` to find
    /// begin blocks at ANY depth within assignment values, not just direct.
    in_begin_assignment: bool,
    /// True when we're traversing an assignment's value subtree. Any BeginNode
    /// encountered while this is true triggers `in_begin_assignment`.
    in_assignment_value: bool,
}

impl NoReturnVisitor<'_, '_> {
    fn check_assignment_value(&mut self, value: &ruby_prism::Node<'_>) {
        let old = self.in_assignment_value;
        self.in_assignment_value = true;
        self.visit(value);
        self.in_assignment_value = old;
    }
}

impl<'pr> Visit<'pr> for NoReturnVisitor<'_, '_> {
    // Simple assignment: x = begin ... end
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    // Or-assignment: x ||= begin ... end
    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_instance_variable_or_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_class_variable_or_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    // And-assignment: x &&= begin ... end
    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_instance_variable_and_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableAndWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_class_variable_and_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableAndWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_global_variable_and_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableAndWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_constant_and_write_node(&mut self, node: &ruby_prism::ConstantAndWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    fn visit_constant_path_and_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathAndWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    // Global variable or-assignment
    fn visit_global_variable_or_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    // Constant or-assignment
    fn visit_constant_or_write_node(&mut self, node: &ruby_prism::ConstantOrWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    // Constant path or-assignment / operator-assignment
    fn visit_constant_path_or_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathOrWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    // Method call assignments: obj.foo &&= / ||= / += begin ... end
    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    // Index/subscript assignments: arr[i] &&= / ||= / += begin ... end
    fn visit_index_and_write_node(&mut self, node: &ruby_prism::IndexAndWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    fn visit_index_or_write_node(&mut self, node: &ruby_prism::IndexOrWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    fn visit_index_operator_write_node(&mut self, node: &ruby_prism::IndexOperatorWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    // Operator assignments: x += begin ... end, x -= begin ... end, etc.
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_instance_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_class_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_global_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_constant_operator_write_node(
        &mut self,
        node: &ruby_prism::ConstantOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_constant_path_operator_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    // When traversing an assignment value subtree, only EXPLICIT begin..end
    // blocks (kwbegin, i.e. begin_keyword_loc is Some) set in_begin_assignment.
    // Implicit BeginNode from rescue clauses in def/block/lambda bodies must
    // NOT trigger this — those are not assignment-context begin blocks.
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if self.in_assignment_value && node.begin_keyword_loc().is_some() {
            let old = self.in_begin_assignment;
            self.in_begin_assignment = true;
            ruby_prism::visit_begin_node(self, node);
            self.in_begin_assignment = old;
        } else {
            ruby_prism::visit_begin_node(self, node);
        }
    }

    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        if self.in_begin_assignment {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not `return` in `begin..end` blocks in assignment contexts.".to_string(),
            ));
        }
    }

    // RuboCop walks assignment subtrees into nested defs, so explicit kwbegin
    // descendants of assignment-valued blocks stay in assignment context even
    // when the `return` is inside a nested method body.
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        ruby_prism::visit_def_node(self, node);
    }
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let old = self.in_begin_assignment;
        self.in_begin_assignment = false;
        ruby_prism::visit_class_node(self, node);
        self.in_begin_assignment = old;
    }
    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let old = self.in_begin_assignment;
        self.in_begin_assignment = false;
        ruby_prism::visit_module_node(self, node);
        self.in_begin_assignment = old;
    }
    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let old = self.in_begin_assignment;
        self.in_begin_assignment = false;
        ruby_prism::visit_lambda_node(self, node);
        self.in_begin_assignment = old;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NoReturnInBeginEndBlocks,
        "cops/lint/no_return_in_begin_end_blocks"
    );
}
