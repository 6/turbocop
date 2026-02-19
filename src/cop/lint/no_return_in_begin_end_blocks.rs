use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

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
    ) {
        let mut visitor = NoReturnVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_begin_assignment: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct NoReturnVisitor<'a, 'src> {
    cop: &'a NoReturnInBeginEndBlocks,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_begin_assignment: bool,
}

impl NoReturnVisitor<'_, '_> {
    fn check_assignment_value(&mut self, value: &ruby_prism::Node<'_>) {
        // Check if the value is a BeginNode (kwbegin)
        if value.as_begin_node().is_some() {
            let old = self.in_begin_assignment;
            self.in_begin_assignment = true;
            self.visit(value);
            self.in_begin_assignment = old;
        } else {
            self.visit(value);
        }
    }
}

impl<'pr> Visit<'pr> for NoReturnVisitor<'_, '_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.check_assignment_value(&node.value());
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value());
    }

    fn visit_class_variable_write_node(
        &mut self,
        node: &ruby_prism::ClassVariableWriteNode<'pr>,
    ) {
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

    fn visit_constant_path_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathWriteNode<'pr>,
    ) {
        self.check_assignment_value(&node.value().into());
    }

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

    // Don't recurse into nested scopes
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NoReturnInBeginEndBlocks,
        "cops/lint/no_return_in_begin_end_blocks"
    );
}
