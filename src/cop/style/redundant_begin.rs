use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for redundant explicit `begin..end` blocks.
///
/// ## Investigation (2026-03-30)
///
/// FN root cause: the original Prism port only handled three contexts:
/// normal `def` bodies, `do..end` block bodies, and simple variable
/// assignments. RuboCop also flags redundant explicit `begin` blocks when they
/// appear as:
/// - the sole body statement of branch bodies (`if`/`else`/`unless`,
///   `case`/`when`/`in`, non-modifier `while`/`until`)
/// - standalone explicit `begin` statements at top level or as single nested
///   statements
/// - assignment values on Prism-only write nodes such as `IndexOrWriteNode`
///   (`foo[bar] ||= begin ... end`)
///
/// Fix: add explicit begin handling for those missing contexts while preserving
/// RuboCop's allowlist for direct method-call arguments/receivers, logical
/// operator operands, post-condition `while`/`until` loops, and top-level or
/// assignment `begin foo rescue nil end` forms.
///
/// ## Investigation (2026-04-01)
///
/// Remaining FN root causes were narrower Prism mismatches:
/// - generic nested `begin` wrappers were skipped too aggressively whenever the
///   single child was another explicit `begin`, even when RuboCop would flag
///   the outer wrapper because the inner `begin` was itself allowable
///   (`begin begin a; b end end`, or inner `begin` with `rescue`)
/// - the Prism-only `RescueModifierNode` allowance was applied to `def` and
///   `do..end` bodies, but RuboCop still flags those bodies because the outer
///   context can absorb the implicit rescue
///
/// Fix: only suppress an outer nested `begin` when the inner subtree contains a
/// non-root generic offense, and keep the rescue-modifier allowance out of
/// `def`/`do..end` body checks.
///
/// ## Investigation (2026-04-01, second pass)
///
/// Resolved 6 of 7 remaining FNs. Two root causes:
/// 1. `visit_begin_children` did not traverse `else_clause()` of `BeginNode`,
///    so any `begin` offenses nested inside `else` branches of
///    `begin..rescue..else..end` blocks were never reached. Fixed by adding
///    else_clause visitation.
/// 2. `visit_index_{or,and,operator}_write_node` only visited `value()`, not
///    `receiver()` or `arguments()`, so `begin` inside splats in array index
///    expressions (e.g. `h[*begin [:k] end] ||= 20`) was unreachable. Fixed
///    by visiting receiver and arguments before checking the value.
///
/// ## Investigation (2026-04-01, third pass)
///
/// The last remaining FN was not a config issue. The corpus file was truncated
/// in the prompt: the real pattern is `@ivar ||= begin ... end&.decorate`.
/// Prism parses this as an assignment whose value is a `CallNode` with the
/// explicit `BeginNode` as its receiver. RuboCop still flags that receiver
/// `begin`, but this visitor treated both call receivers and direct call
/// arguments as "allowed direct begin children", which skipped the offense.
///
/// Fix: inspect `CallNode` receivers normally so chained-call receiver begins
/// still flow through generic `begin` detection, while keeping the direct
/// method-argument allowance for `do_something begin ... end`.
///
/// ## Investigation (2026-04-03)
///
/// Corpus FP cluster: RuboCop allows explicit `begin` when it is the receiver
/// of a regular chained call like `begin ... end.freeze`, `end.to_sym`, or
/// `end.round` under the corpus baseline config. The previous pass regressed by
/// treating every call receiver like the safe-navigation FN above, which made
/// plain `.` receivers look redundant in assignments and method bodies.
///
/// Fix: allow `BeginNode` receivers for regular `CallNode`s again, but only
/// when the call operator is not `&.`. Safe-navigation receivers still flow
/// through generic `begin` detection so `begin ... end&.decorate` remains an
/// offense.
pub struct RedundantBegin;

impl Cop for RedundantBegin {
    fn name(&self) -> &'static str {
        "Style/RedundantBegin"
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
        let mut visitor = RedundantBeginVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RedundantBeginVisitor<'a> {
    cop: &'a RedundantBegin,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl RedundantBeginVisitor<'_> {
    fn add_begin_offense(&mut self, begin_node: &ruby_prism::BeginNode<'_>) {
        let Some(begin_kw_loc) = begin_node.begin_keyword_loc() else {
            return;
        };

        let offset = begin_kw_loc.start_offset();
        let (line, column) = self.source.offset_to_line_col(offset);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Redundant `begin` block detected.".to_string(),
        ));
    }

    fn begin_body_nodes<'pr>(
        begin_node: &ruby_prism::BeginNode<'pr>,
    ) -> Vec<ruby_prism::Node<'pr>> {
        begin_node
            .statements()
            .map(|stmts| stmts.body().iter().collect())
            .unwrap_or_default()
    }

    fn visit_begin_children<'pr>(&mut self, begin_node: &ruby_prism::BeginNode<'pr>) {
        if let Some(stmts) = begin_node.statements() {
            for child in stmts.body().iter() {
                self.visit(&child);
            }
        }
        if let Some(rescue) = begin_node.rescue_clause() {
            self.visit_rescue_node(&rescue);
        }
        if let Some(else_clause) = begin_node.else_clause() {
            self.visit(&else_clause.as_node());
        }
        if let Some(ensure) = begin_node.ensure_clause() {
            self.visit_ensure_node(&ensure);
        }
    }

    fn body_is_allowable_rescue_modifier(body_nodes: &[ruby_prism::Node<'_>]) -> bool {
        if body_nodes.len() != 1 {
            return false;
        }

        let Some(rescue_modifier) = body_nodes[0].as_rescue_modifier_node() else {
            return false;
        };

        let expression = rescue_modifier.expression();
        expression.as_if_node().is_none() && expression.as_unless_node().is_none()
    }

    fn is_non_root_generic_begin_offense(begin_node: &ruby_prism::BeginNode<'_>) -> bool {
        if begin_node.begin_keyword_loc().is_none()
            || begin_node.rescue_clause().is_some()
            || begin_node.ensure_clause().is_some()
            || begin_node.else_clause().is_some()
        {
            return false;
        }

        let body_nodes = Self::begin_body_nodes(begin_node);
        if body_nodes.is_empty() || body_nodes.len() != 1 {
            return false;
        }

        !Self::body_is_allowable_rescue_modifier(&body_nodes)
    }

    fn has_non_root_generic_begin_offense(begin_node: &ruby_prism::BeginNode<'_>) -> bool {
        if Self::is_non_root_generic_begin_offense(begin_node) {
            return true;
        }

        Self::begin_body_nodes(begin_node).into_iter().any(|child| {
            child
                .as_begin_node()
                .is_some_and(|inner| Self::has_non_root_generic_begin_offense(&inner))
        })
    }

    fn inspect_generic_begin<'pr>(
        &mut self,
        begin_node: &ruby_prism::BeginNode<'pr>,
        root_program_begin: bool,
    ) {
        if begin_node.begin_keyword_loc().is_none()
            || begin_node.rescue_clause().is_some()
            || begin_node.ensure_clause().is_some()
            || begin_node.else_clause().is_some()
        {
            self.visit_begin_children(begin_node);
            return;
        }

        let body_nodes = Self::begin_body_nodes(begin_node);
        if body_nodes.is_empty() {
            return;
        }

        if Self::body_is_allowable_rescue_modifier(&body_nodes) {
            self.visit_begin_children(begin_node);
            return;
        }

        // RuboCop's kwbegin search prefers the deepest offensive begin, so an
        // outer `begin` that only wraps another subtree with its own generic
        // offense does not fire.
        if body_nodes.len() == 1
            && body_nodes[0]
                .as_begin_node()
                .is_some_and(|inner| Self::has_non_root_generic_begin_offense(&inner))
        {
            self.visit_begin_children(begin_node);
            return;
        }

        if root_program_begin || body_nodes.len() == 1 {
            self.add_begin_offense(begin_node);
        }

        self.visit_begin_children(begin_node);
    }

    fn inspect_branch_statements<'pr>(
        &mut self,
        statements: Option<ruby_prism::StatementsNode<'pr>>,
    ) {
        let Some(statements) = statements else {
            return;
        };

        let body_nodes: Vec<_> = statements.body().iter().collect();
        if body_nodes.len() != 1 {
            for child in body_nodes {
                self.visit(&child);
            }
            return;
        }

        let Some(begin_node) = body_nodes[0].as_begin_node() else {
            self.visit(&body_nodes[0]);
            return;
        };

        if begin_node.begin_keyword_loc().is_none()
            || begin_node.rescue_clause().is_some()
            || begin_node.ensure_clause().is_some()
            || begin_node.else_clause().is_some()
            || Self::body_is_allowable_rescue_modifier(&Self::begin_body_nodes(&begin_node))
        {
            self.visit_begin_children(&begin_node);
            return;
        }

        self.add_begin_offense(&begin_node);
        self.visit_begin_children(&begin_node);
    }

    fn visit_post_condition_loop_statements<'pr>(
        &mut self,
        statements: Option<ruby_prism::StatementsNode<'pr>>,
    ) {
        let Some(statements) = statements else {
            return;
        };

        let body_nodes: Vec<_> = statements.body().iter().collect();
        if body_nodes.len() == 1 {
            if let Some(begin_node) = body_nodes[0].as_begin_node() {
                if begin_node.begin_keyword_loc().is_some() {
                    self.visit_begin_children(&begin_node);
                    return;
                }
            }
        }

        for child in body_nodes {
            self.visit(&child);
        }
    }

    fn check_body_begin(&mut self, body: Option<ruby_prism::Node<'_>>) {
        let body = match body {
            Some(body) => body,
            None => return,
        };

        let begin_node = if let Some(begin_node) = body.as_begin_node() {
            begin_node
        } else if let Some(statements) = body.as_statements_node() {
            let body_nodes: Vec<_> = statements.body().iter().collect();
            if body_nodes.len() != 1 {
                for child in body_nodes {
                    self.visit(&child);
                }
                return;
            }

            let Some(begin_node) = body_nodes[0].as_begin_node() else {
                self.visit(&body_nodes[0]);
                return;
            };
            begin_node
        } else {
            self.visit(&body);
            return;
        };

        if begin_node.begin_keyword_loc().is_none() {
            self.visit_begin_children(&begin_node);
            return;
        }

        self.add_begin_offense(&begin_node);
        self.visit_begin_children(&begin_node);
    }

    fn check_assignment_begin(&mut self, value: &ruby_prism::Node<'_>) {
        let Some(begin_node) = value.as_begin_node() else {
            self.visit(value);
            return;
        };

        if begin_node.begin_keyword_loc().is_none()
            || begin_node.rescue_clause().is_some()
            || begin_node.ensure_clause().is_some()
            || begin_node.else_clause().is_some()
        {
            self.visit_begin_children(&begin_node);
            return;
        }

        let body_nodes = Self::begin_body_nodes(&begin_node);
        if Self::body_is_allowable_rescue_modifier(&body_nodes) {
            self.visit_begin_children(&begin_node);
            return;
        }

        if body_nodes.len() != 1 {
            self.visit_begin_children(&begin_node);
            return;
        }

        self.add_begin_offense(&begin_node);
        self.visit_begin_children(&begin_node);
    }

    fn visit_allowed_direct_begin_child(&mut self, node: &ruby_prism::Node<'_>) {
        let Some(begin_node) = node.as_begin_node() else {
            self.visit(node);
            return;
        };

        if begin_node.begin_keyword_loc().is_none() {
            self.visit(node);
            return;
        }

        self.visit_begin_children(&begin_node);
    }
}

impl<'pr> Visit<'pr> for RedundantBeginVisitor<'_> {
    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        let body_nodes: Vec<_> = node.statements().body().iter().collect();
        if body_nodes.len() == 1 {
            if let Some(begin_node) = body_nodes[0].as_begin_node() {
                self.inspect_generic_begin(&begin_node, true);
                return;
            }
        }

        for child in body_nodes {
            self.visit(&child);
        }
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let is_endless = node.end_keyword_loc().is_none() && node.equal_loc().is_some();
        if is_endless {
            if let Some(body) = node.body() {
                self.visit(&body);
            }
            return;
        }

        self.check_body_begin(node.body());
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        self.inspect_generic_begin(node, false);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        if node.opening_loc().as_slice() == b"{" {
            if let Some(body) = node.body() {
                self.visit(&body);
            }
            return;
        }

        self.check_body_begin(node.body());
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.visit(&node.predicate());

        if node.end_keyword_loc().is_none() {
            if let Some(statements) = node.statements() {
                self.visit(&statements.as_node());
            }
        } else {
            self.inspect_branch_statements(node.statements());
        }

        if let Some(subsequent) = node.subsequent() {
            self.visit(&subsequent);
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.visit(&node.predicate());

        if node.end_keyword_loc().is_none() {
            if let Some(statements) = node.statements() {
                self.visit(&statements.as_node());
            }
        } else {
            self.inspect_branch_statements(node.statements());
        }

        if let Some(else_clause) = node.else_clause() {
            self.visit(&else_clause.as_node());
        }
    }

    fn visit_else_node(&mut self, node: &ruby_prism::ElseNode<'pr>) {
        self.inspect_branch_statements(node.statements());
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        if let Some(predicate) = node.predicate() {
            self.visit(&predicate);
        }
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit(&else_clause.as_node());
        }
    }

    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit(&else_clause.as_node());
        }
    }

    fn visit_when_node(&mut self, node: &ruby_prism::WhenNode<'pr>) {
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        self.inspect_branch_statements(node.statements());
    }

    fn visit_in_node(&mut self, node: &ruby_prism::InNode<'pr>) {
        self.inspect_branch_statements(node.statements());
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        self.visit(&node.predicate());

        if node.is_begin_modifier() {
            self.visit_post_condition_loop_statements(node.statements());
            return;
        }

        self.inspect_branch_statements(node.statements());
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        self.visit(&node.predicate());

        if node.is_begin_modifier() {
            self.visit_post_condition_loop_statements(node.statements());
            return;
        }

        self.inspect_branch_statements(node.statements());
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            if node.is_safe_navigation() {
                self.visit(&receiver);
            } else {
                self.visit_allowed_direct_begin_child(&receiver);
            }
        }
        if let Some(arguments) = node.arguments() {
            for argument in arguments.arguments().iter() {
                self.visit_allowed_direct_begin_child(&argument);
            }
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        self.visit_allowed_direct_begin_child(&node.left());
        self.visit_allowed_direct_begin_child(&node.right());
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        self.visit_allowed_direct_begin_child(&node.left());
        self.visit_allowed_direct_begin_child(&node.right());
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_instance_variable_or_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_class_variable_or_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_global_variable_or_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_or_write_node(&mut self, node: &ruby_prism::ConstantOrWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_path_or_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathOrWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_instance_variable_and_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableAndWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_class_variable_and_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableAndWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_global_variable_and_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableAndWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_and_write_node(&mut self, node: &ruby_prism::ConstantAndWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_path_and_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathAndWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_instance_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_class_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_global_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_operator_write_node(
        &mut self,
        node: &ruby_prism::ConstantOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_constant_path_operator_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathOperatorWriteNode<'pr>,
    ) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_call_or_write_node(&mut self, node: &ruby_prism::CallOrWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_call_and_write_node(&mut self, node: &ruby_prism::CallAndWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_call_operator_write_node(&mut self, node: &ruby_prism::CallOperatorWriteNode<'pr>) {
        self.check_assignment_begin(&node.value());
    }

    fn visit_index_or_write_node(&mut self, node: &ruby_prism::IndexOrWriteNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
        }
        if let Some(arguments) = node.arguments() {
            for argument in arguments.arguments().iter() {
                self.visit(&argument);
            }
        }
        self.check_assignment_begin(&node.value());
    }

    fn visit_index_and_write_node(&mut self, node: &ruby_prism::IndexAndWriteNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
        }
        if let Some(arguments) = node.arguments() {
            for argument in arguments.arguments().iter() {
                self.visit(&argument);
            }
        }
        self.check_assignment_begin(&node.value());
    }

    fn visit_index_operator_write_node(&mut self, node: &ruby_prism::IndexOperatorWriteNode<'pr>) {
        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
        }
        if let Some(arguments) = node.arguments() {
            for argument in arguments.arguments().iter() {
                self.visit(&argument);
            }
        }
        self.check_assignment_begin(&node.value());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantBegin, "cops/style/redundant_begin");
}
