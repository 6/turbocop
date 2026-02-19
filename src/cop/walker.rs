use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CopWalker<'a, 'pr> {
    pub cop: &'a dyn Cop,
    pub source: &'a SourceFile,
    pub parse_result: &'a ruby_prism::ParseResult<'pr>,
    pub cop_config: &'a CopConfig,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for CopWalker<'_, 'pr> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        let results =
            self.cop
                .check_node(self.source, &node, self.parse_result, self.cop_config);
        self.diagnostics.extend(results);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        let results =
            self.cop
                .check_node(self.source, &node, self.parse_result, self.cop_config);
        self.diagnostics.extend(results);
    }
}

/// Walks the AST once and dispatches every node to all enabled cops.
/// Eliminates N separate tree traversals (one per cop) in favor of a single
/// traversal with N cop calls at each node.
pub struct BatchedCopWalker<'a, 'pr> {
    pub cops: Vec<(&'a dyn Cop, &'a CopConfig)>,
    pub source: &'a SourceFile,
    pub parse_result: &'a ruby_prism::ParseResult<'pr>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for BatchedCopWalker<'_, 'pr> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        for &(cop, cop_config) in &self.cops {
            let results = cop.check_node(self.source, &node, self.parse_result, cop_config);
            self.diagnostics.extend(results);
        }
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        for &(cop, cop_config) in &self.cops {
            let results = cop.check_node(self.source, &node, self.parse_result, cop_config);
            self.diagnostics.extend(results);
        }
    }
}
