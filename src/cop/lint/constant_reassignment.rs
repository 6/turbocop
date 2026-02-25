use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ConstantReassignment;

impl Cop for ConstantReassignment {
    fn name(&self) -> &'static str {
        "Lint/ConstantReassignment"
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
        let mut visitor = ConstantReassignmentVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            seen_constants: HashSet::new(),
            namespace_stack: Vec::new(),
            conditional_depth: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ConstantReassignmentVisitor<'a, 'src> {
    cop: &'a ConstantReassignment,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    seen_constants: HashSet<String>,
    namespace_stack: Vec<String>,
    /// Tracks nesting depth inside conditional/rescue/block contexts.
    /// When > 0, constant writes are not counted as reassignments because
    /// they may be conditional first-time assignments (e.g., rescue fallbacks,
    /// if/else branches).
    conditional_depth: usize,
}

impl ConstantReassignmentVisitor<'_, '_> {
    fn fully_qualified_name(&self, name: &str) -> String {
        let mut parts = Vec::new();
        for ns in &self.namespace_stack {
            parts.push(ns.as_str());
        }
        parts.push(name);
        format!("::{}", parts.join("::"))
    }
}

impl<'pr> Visit<'pr> for ConstantReassignmentVisitor<'_, '_> {
    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        if self.conditional_depth == 0 {
            let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
            let fqn = self.fully_qualified_name(name);

            if !self.seen_constants.insert(fqn) {
                let loc = node.name_loc();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    format!("Constant `{name}` is already assigned in this namespace."),
                ));
            }
        }

        ruby_prism::visit_constant_write_node(self, node);
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.namespace_stack.push(name);
        ruby_prism::visit_class_node(self, node);
        self.namespace_stack.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.namespace_stack.push(name);
        ruby_prism::visit_module_node(self, node);
        self.namespace_stack.pop();
    }

    // Conditional/branching contexts: constant assignments inside these
    // are not unconditional reassignments.

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_if_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_unless_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_case_node(self, node);
        self.conditional_depth -= 1;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        // begin...rescue blocks: the rescue clause means assignment may be conditional
        if node.rescue_clause().is_some() {
            self.conditional_depth += 1;
            ruby_prism::visit_begin_node(self, node);
            self.conditional_depth -= 1;
        } else {
            ruby_prism::visit_begin_node(self, node);
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.conditional_depth += 1;
        ruby_prism::visit_block_node(self, node);
        self.conditional_depth -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantReassignment, "cops/lint/constant_reassignment");
}
