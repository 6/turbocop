use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ConstantDefinitionInBlock;

impl Cop for ConstantDefinitionInBlock {
    fn name(&self) -> &'static str {
        "Lint/ConstantDefinitionInBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let allowed_methods = config
            .get_string_array("AllowedMethods")
            .unwrap_or_else(|| vec!["enums".to_string()]);
        let mut visitor = BlockConstVisitor {
            cop: self,
            source,
            allowed_methods,
            block_depth: 0,
            current_block_method: Vec::new(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct BlockConstVisitor<'a, 'src> {
    cop: &'a ConstantDefinitionInBlock,
    source: &'src SourceFile,
    allowed_methods: Vec<String>,
    block_depth: usize,
    current_block_method: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

impl BlockConstVisitor<'_, '_> {
    fn in_block(&self) -> bool {
        self.block_depth > 0
    }

    fn current_method_allowed(&self) -> bool {
        if let Some(method_name) = self.current_block_method.last() {
            self.allowed_methods.iter().any(|a| a == method_name)
        } else {
            false
        }
    }
}

impl<'pr> Visit<'pr> for BlockConstVisitor<'_, '_> {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.block_depth += 1;
        // Don't push a method name here -- the call_node is the parent
        ruby_prism::visit_block_node(self, node);
        self.block_depth -= 1;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // If this call has a block, record the method name
        if node.block().is_some() {
            let name = std::str::from_utf8(node.name().as_slice())
                .unwrap_or("")
                .to_string();
            self.current_block_method.push(name);
            ruby_prism::visit_call_node(self, node);
            self.current_block_method.pop();
        } else {
            ruby_prism::visit_call_node(self, node);
        }
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        if self.in_block() && !self.current_method_allowed() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not define constants this way within a block.".to_string(),
            ));
        }
        ruby_prism::visit_constant_write_node(self, node);
    }

    fn visit_constant_path_write_node(
        &mut self,
        node: &ruby_prism::ConstantPathWriteNode<'pr>,
    ) {
        // RuboCop only flags bare constant assignments (FOO = 1) in blocks,
        // not namespaced ones (::FOO = 1 or Mod::FOO = 1). The RuboCop
        // pattern uses `nil?` for the namespace, which excludes all
        // ConstantPathWriteNode cases. Namespaced constant writes explicitly
        // scope the constant, so they are intentional.
        ruby_prism::visit_constant_path_write_node(self, node);
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if self.in_block() && !self.current_method_allowed() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not define constants this way within a block.".to_string(),
            ));
            // Don't recurse into class body for more constant defs
            return;
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if self.in_block() && !self.current_method_allowed() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not define constants this way within a block.".to_string(),
            ));
            return;
        }
        ruby_prism::visit_module_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantDefinitionInBlock, "cops/lint/constant_definition_in_block");
}
