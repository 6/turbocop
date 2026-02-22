use ruby_prism::Visit;

use crate::cop::node_type::{BEGIN_NODE, RETURN_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EnsureReturn;

struct ReturnFinder {
    found: Vec<usize>,
}

impl<'pr> Visit<'pr> for ReturnFinder {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        if node.as_return_node().is_some() {
            self.found.push(node.location().start_offset());
        }
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        if node.as_return_node().is_some() {
            self.found.push(node.location().start_offset());
        }
    }
}

impl Cop for EnsureReturn {
    fn name(&self) -> &'static str {
        "Lint/EnsureReturn"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, RETURN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // EnsureNode is visited via visit_begin_node's specific method,
        // not via the generic visit() dispatch. So we match BeginNode
        // and check its ensure_clause.
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return,
        };

        let ensure_node = match begin_node.ensure_clause() {
            Some(n) => n,
            None => return,
        };

        let statements = match ensure_node.statements() {
            Some(s) => s,
            None => return,
        };

        let mut finder = ReturnFinder { found: vec![] };
        for stmt in statements.body().iter() {
            finder.visit(&stmt);
        }

        diagnostics.extend(finder.found.iter().map(|&offset| {
            let (line, column) = source.offset_to_line_col(offset);
            self.diagnostic(
                source,
                line,
                column,
                "Do not return from an `ensure` block.".to_string(),
            )
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EnsureReturn, "cops/lint/ensure_return");
}
