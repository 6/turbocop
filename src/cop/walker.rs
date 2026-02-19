use ruby_prism::Visit;

use crate::cop::node_type::{node_type_tag, NODE_TYPE_COUNT};
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
        self.cop.check_node(
            self.source,
            &node,
            self.parse_result,
            self.cop_config,
            &mut self.diagnostics,
        );
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.cop.check_node(
            self.source,
            &node,
            self.parse_result,
            self.cop_config,
            &mut self.diagnostics,
        );
    }
}

/// Walks the AST once and dispatches each node only to cops that declared
/// interest in that node type. Cops that haven't declared interest (empty
/// `interested_node_types()`) are called for every node (universal dispatch).
pub struct BatchedCopWalker<'a, 'pr> {
    /// Cops that haven't declared node type interest — called for every node.
    universal_cops: Vec<(&'a dyn Cop, &'a CopConfig)>,
    /// Dispatch table: indexed by node type tag, each entry = cops for that type.
    dispatch_table: [Vec<(&'a dyn Cop, &'a CopConfig)>; NODE_TYPE_COUNT],
    pub source: &'a SourceFile,
    pub parse_result: &'a ruby_prism::ParseResult<'pr>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a, 'pr> BatchedCopWalker<'a, 'pr> {
    pub fn new(
        cops: Vec<(&'a dyn Cop, &'a CopConfig)>,
        source: &'a SourceFile,
        parse_result: &'a ruby_prism::ParseResult<'pr>,
    ) -> Self {
        let mut universal = Vec::new();
        let mut table: [Vec<(&'a dyn Cop, &'a CopConfig)>; NODE_TYPE_COUNT] =
            std::array::from_fn(|_| Vec::new());

        for (cop, config) in cops {
            let types = cop.interested_node_types();
            if types.is_empty() {
                universal.push((cop, config));
            } else {
                for &t in types {
                    table[t as usize].push((cop, config));
                }
            }
        }

        Self {
            universal_cops: universal,
            dispatch_table: table,
            source,
            parse_result,
            diagnostics: Vec::new(),
        }
    }

    #[inline]
    fn dispatch(&mut self, node: &ruby_prism::Node<'pr>) {
        let tag = node_type_tag(node) as usize;

        for &(cop, cop_config) in &self.universal_cops {
            cop.check_node(self.source, node, self.parse_result, cop_config, &mut self.diagnostics);
        }

        if let Some(cops) = self.dispatch_table.get(tag) {
            for &(cop, cop_config) in cops {
                cop.check_node(self.source, node, self.parse_result, cop_config, &mut self.diagnostics);
            }
        }
    }
}

impl<'pr> Visit<'pr> for BatchedCopWalker<'_, 'pr> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.dispatch(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.dispatch(&node);
    }
}
