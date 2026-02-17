use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantMatch;

impl Cop for RedundantMatch {
    fn name(&self) -> &'static str {
        "Performance/RedundantMatch"
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
    ) -> Vec<Diagnostic> {
        use ruby_prism::Visit;
        let mut visitor = RedundantMatchVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            parent_is_condition: false,
            value_used: false,
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct RedundantMatchVisitor<'a> {
    cop: &'a RedundantMatch,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Whether the current node position is a condition of an if/while/until/case
    parent_is_condition: bool,
    /// Whether the result value is used (assignment, argument, return, etc.)
    value_used: bool,
}

impl<'pr> ruby_prism::Visit<'pr> for RedundantMatchVisitor<'_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        // The predicate is the condition - only truthiness matters there
        let old_condition = self.parent_is_condition;
        let old_used = self.value_used;

        self.parent_is_condition = true;
        self.value_used = false;
        self.visit(&node.predicate());
        self.parent_is_condition = false;

        // Branches: value may be used depending on context, be conservative
        self.value_used = true;
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
        if let Some(subsequent) = node.subsequent() {
            self.visit(&subsequent);
        }

        self.parent_is_condition = old_condition;
        self.value_used = old_used;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let old_condition = self.parent_is_condition;
        let old_used = self.value_used;

        self.parent_is_condition = true;
        self.value_used = false;
        self.visit(&node.predicate());
        self.parent_is_condition = false;

        self.value_used = true;
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit(&else_clause.as_node());
        }

        self.parent_is_condition = old_condition;
        self.value_used = old_used;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        let old_condition = self.parent_is_condition;
        let old_used = self.value_used;

        self.parent_is_condition = true;
        self.value_used = false;
        self.visit(&node.predicate());
        self.parent_is_condition = false;

        self.value_used = false;
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }

        self.parent_is_condition = old_condition;
        self.value_used = old_used;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let old_condition = self.parent_is_condition;
        let old_used = self.value_used;

        self.parent_is_condition = true;
        self.value_used = false;
        self.visit(&node.predicate());
        self.parent_is_condition = false;

        self.value_used = false;
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }

        self.parent_is_condition = old_condition;
        self.value_used = old_used;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let old_condition = self.parent_is_condition;
        let old_used = self.value_used;

        if let Some(pred) = node.predicate() {
            self.parent_is_condition = true;
            self.value_used = false;
            self.visit(&pred);
            self.parent_is_condition = false;
        }

        self.value_used = true;
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        if let Some(else_clause) = node.else_clause() {
            self.visit(&else_clause.as_node());
        }

        self.parent_is_condition = old_condition;
        self.value_used = old_used;
    }

    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let old_used = self.value_used;
        let stmts: Vec<_> = node.body().iter().collect();
        for (i, stmt) in stmts.iter().enumerate() {
            // In a statement list, only the last statement's value might be used
            // (e.g., as block return value). Earlier statements are not used.
            self.value_used = if i == stmts.len() - 1 { old_used } else { false };
            self.visit(stmt);
        }
        self.value_used = old_used;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"match" {
            self.check_match_call(node);
        }

        // Visit children with value_used context
        let old_used = self.value_used;
        let old_condition = self.parent_is_condition;

        // Receiver: value is used (as receiver)
        self.value_used = true;
        self.parent_is_condition = false;
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }

        // Arguments: values are used
        self.value_used = true;
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }

        // Block: the last expression's value may be used as block return
        if let Some(block) = node.block() {
            self.value_used = true;
            self.visit(&block);
        }

        self.value_used = old_used;
        self.parent_is_condition = old_condition;
    }

    fn visit_local_variable_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableWriteNode<'pr>,
    ) {
        // The RHS of an assignment: value IS used
        let old = self.value_used;
        let old_condition = self.parent_is_condition;
        self.value_used = true;
        self.parent_is_condition = false;
        self.visit(&node.value());
        self.value_used = old;
        self.parent_is_condition = old_condition;
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        let old = self.value_used;
        let old_condition = self.parent_is_condition;
        self.value_used = true;
        self.parent_is_condition = false;
        self.visit(&node.value());
        self.value_used = old;
        self.parent_is_condition = old_condition;
    }

    fn visit_class_variable_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableWriteNode<'pr>,
    ) {
        let old = self.value_used;
        let old_condition = self.parent_is_condition;
        self.value_used = true;
        self.parent_is_condition = false;
        self.visit(&node.value());
        self.value_used = old;
        self.parent_is_condition = old_condition;
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        let old = self.value_used;
        let old_condition = self.parent_is_condition;
        self.value_used = true;
        self.parent_is_condition = false;
        self.visit(&node.value());
        self.value_used = old;
        self.parent_is_condition = old_condition;
    }

    fn visit_constant_write_node(
        &mut self,
        node: &ruby_prism::ConstantWriteNode<'pr>,
    ) {
        let old = self.value_used;
        let old_condition = self.parent_is_condition;
        self.value_used = true;
        self.parent_is_condition = false;
        self.visit(&node.value());
        self.value_used = old;
        self.parent_is_condition = old_condition;
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        // ||= assignment: value IS used
        let old = self.value_used;
        let old_condition = self.parent_is_condition;
        self.value_used = true;
        self.parent_is_condition = false;
        self.visit(&node.value());
        self.value_used = old;
        self.parent_is_condition = old_condition;
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        // &&= assignment: value IS used
        let old = self.value_used;
        let old_condition = self.parent_is_condition;
        self.value_used = true;
        self.parent_is_condition = false;
        self.visit(&node.value());
        self.value_used = old;
        self.parent_is_condition = old_condition;
    }

    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        // return value IS used
        let old = self.value_used;
        self.value_used = true;
        self.parent_is_condition = false;
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }
        self.value_used = old;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let old = self.value_used;
        // Block body: last expression's value may be used as block return
        self.value_used = true;
        self.parent_is_condition = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.value_used = old;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let old = self.value_used;
        let old_condition = self.parent_is_condition;
        // Method body: last expression IS the return value
        self.value_used = true;
        self.parent_is_condition = false;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.value_used = old;
        self.parent_is_condition = old_condition;
    }
}

impl<'a> RedundantMatchVisitor<'a> {
    fn check_match_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // RuboCop uses RESTRICT_ON_SEND = %i[match], which only matches regular
        // method calls (send), NOT safe-navigation calls (csend / &.match).
        if let Some(op) = call.call_operator_loc() {
            let bytes = &self.source.as_bytes()[op.start_offset()..op.end_offset()];
            if bytes == b"&." {
                return;
            }
        }

        // Must have a receiver (x.match)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Must have arguments (x.match(y))
        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // RuboCop only flags when a string or regexp literal appears on one side
        let first_arg = match arguments.arguments().iter().next() {
            Some(a) => a,
            None => return,
        };

        let recv_is_literal = receiver.as_string_node().is_some()
            || receiver.as_regular_expression_node().is_some();
        let arg_is_literal = first_arg.as_string_node().is_some()
            || first_arg.as_regular_expression_node().is_some();

        if !recv_is_literal && !arg_is_literal {
            return;
        }

        // Don't flag if the call has a block
        if call.block().is_some() {
            return;
        }

        // Only flag when:
        // 1. The value is not used at all, OR
        // 2. Only truthiness matters (it's the condition of an if/while/until/case)
        if self.value_used && !self.parent_is_condition {
            return;
        }

        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `match?` instead of `match` when `MatchData` is not used.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantMatch, "cops/performance/redundant_match");
}
