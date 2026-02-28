use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ReverseEach;

impl Cop for ReverseEach {
    fn name(&self) -> &'static str {
        "Performance/ReverseEach"
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
        let mut visitor = ReverseEachVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            value_used: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ReverseEachVisitor<'a, 'src> {
    cop: &'a ReverseEach,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Whether the current expression's return value is used by a parent.
    value_used: bool,
}

impl<'a, 'src> ReverseEachVisitor<'a, 'src> {
    fn check_reverse_each(&mut self, node: &ruby_prism::CallNode<'_>) {
        let outer_method = node.name().as_slice();
        if outer_method != b"each" {
            return;
        }

        let receiver = match node.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"reverse" {
            return;
        }

        // Skip if the return value is used — reverse.each and reverse_each
        // return different values ([3,2,1] vs [1,2,3]).
        if self.value_used {
            return;
        }

        // Report at the `reverse.each` range (from `reverse` selector to `each` selector end).
        let start_offset = inner_call.message_loc().map_or_else(
            || inner_call.location().start_offset(),
            |loc| loc.start_offset(),
        );
        let (line, column) = self.source.offset_to_line_col(start_offset);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `reverse_each` instead of `reverse.each`.".to_string(),
        ));
    }

    /// Helper: visit a node with value_used set to a given value, then restore.
    fn visit_with_used(&mut self, node: &ruby_prism::Node<'_>, used: bool) {
        let prev = self.value_used;
        self.value_used = used;
        self.visit(node);
        self.value_used = prev;
    }
}

impl<'pr> Visit<'pr> for ReverseEachVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check this call for reverse.each offense
        self.check_reverse_each(node);

        // Receiver's value is used (as the receiver of a method call).
        // This also means if our reverse.each is itself a receiver of another
        // call (e.g., arr.reverse.each.with_index), it will be marked as value_used.
        if let Some(recv) = node.receiver() {
            self.visit_with_used(&recv, true);
        }

        // Arguments' values are used
        if let Some(args) = node.arguments() {
            let prev = self.value_used;
            self.value_used = true;
            self.visit_arguments_node(&args);
            self.value_used = prev;
        }

        // Block inherits parent context (a block body may or may not have its
        // value used, but conservatively we don't change value_used here since
        // reverse.each inside a block body should still be flagged unless
        // the block is part of a larger call chain).
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    // Assignment nodes: RHS value is used
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_instance_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_class_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_global_variable_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_constant_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_constant_path_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_multi_write_node(self, node);
        self.value_used = prev;
    }

    // Operator-assignment nodes (+=, ||=, &&=, etc.)
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_operator_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_or_write_node(self, node);
        self.value_used = prev;
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_local_variable_and_write_node(self, node);
        self.value_used = prev;
    }

    // Return node: returned values are used
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        let prev = self.value_used;
        self.value_used = true;
        ruby_prism::visit_return_node(self, node);
        self.value_used = prev;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(ReverseEach, "cops/performance/reverse_each");
}
