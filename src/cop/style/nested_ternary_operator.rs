use ruby_prism::Visit;

use crate::cop::shared::node_type::IF_NODE;
use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for nested ternary operator expressions.
///
/// RuboCop uses `node.each_descendant(:if).select(&:ternary?)` to find nested
/// ternaries anywhere inside an outer ternary — including inside string
/// interpolations, method arguments, array literals, and block bodies.
///
/// The original implementation only checked immediate statement children and
/// parentheses, missing ternaries nested inside interpolations, call args,
/// arrays, blocks, etc. Fixed by using a Visit-based recursive descendant
/// search that mirrors RuboCop's `each_descendant` behavior.
pub struct NestedTernaryOperator;

/// Visitor that collects all ternary IfNodes among descendants, skipping the root node.
struct TernaryFinder<'a> {
    source: &'a SourceFile,
    results: Vec<(usize, usize)>,
    root_offset: usize,
}

impl<'pr> Visit<'pr> for TernaryFinder<'_> {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        if util::is_ternary(node) && node.location().start_offset() != self.root_offset {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.results.push((line, column));
        }
        // Continue visiting children to find deeper nesting
        ruby_prism::visit_if_node(self, node);
    }
}

impl Cop for NestedTernaryOperator {
    fn name(&self) -> &'static str {
        "Style/NestedTernaryOperator"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE]
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
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Must be a ternary
        if !util::is_ternary(&if_node) {
            return;
        }

        // Search all descendants for nested ternary operators,
        // mirroring RuboCop's `node.each_descendant(:if).select(&:ternary?)`
        let mut finder = TernaryFinder {
            source,
            results: Vec::new(),
            root_offset: if_node.location().start_offset(),
        };
        finder.visit_if_node(&if_node);

        for (line, column) in finder.results {
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Ternary operators must not be nested. Prefer `if` or `else` constructs instead.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedTernaryOperator, "cops/style/nested_ternary_operator");
}
