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
