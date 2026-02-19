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
            direct_in_block: false,
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
    /// Whether the current node is a direct child of a block body.
    /// RuboCop's pattern `{^any_block [^begin ^^any_block]}` means the node
    /// must be either: (a) a direct child of a block node, or (b) a direct
    /// child of a `begin`/StatementsNode that is itself a direct child of a
    /// block node. An `if`/`unless`/etc. between the block and the constant
    /// definition breaks this direct-child relationship.
    direct_in_block: bool,
    current_block_method: Vec<String>,
    diagnostics: Vec<Diagnostic>,
}

impl BlockConstVisitor<'_, '_> {
    fn current_method_allowed(&self) -> bool {
        if let Some(method_name) = self.current_block_method.last() {
            self.allowed_methods.iter().any(|a| a == method_name)
        } else {
            false
        }
    }

    fn should_flag(&self) -> bool {
        self.direct_in_block && !self.current_method_allowed()
    }
}

impl<'pr> Visit<'pr> for BlockConstVisitor<'_, '_> {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = true;
        ruby_prism::visit_block_node(self, node);
        self.direct_in_block = old;
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

    // StatementsNode is transparent — it corresponds to `begin` in Parser gem,
    // which the RuboCop pattern considers transparent via `[^begin ^^any_block]`.
    // So we do NOT reset `direct_in_block` when entering StatementsNode.

    // All compound nodes that can contain statements reset `direct_in_block`
    // because they break the direct parent relationship with the block.
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_if_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_unless_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_while_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_until_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_for_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_case_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_begin_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_rescue_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_ensure_node(&mut self, node: &ruby_prism::EnsureNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_ensure_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_def_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        if self.should_flag() {
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
        if self.should_flag() {
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
        // Reset direct_in_block when entering class body
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_class_node(self, node);
        self.direct_in_block = old;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if self.should_flag() {
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
        // Reset direct_in_block when entering module body
        let old = self.direct_in_block;
        self.direct_in_block = false;
        ruby_prism::visit_module_node(self, node);
        self.direct_in_block = old;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantDefinitionInBlock, "cops/lint/constant_definition_in_block");
}
