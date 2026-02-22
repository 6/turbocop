use ruby_prism::Visit;

use crate::cop::node_type::{AND_NODE, OR_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct UnlessLogicalOperators;

impl Cop for UnlessLogicalOperators {
    fn name(&self) -> &'static str {
        "Style/UnlessLogicalOperators"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, OR_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "forbid_mixed_logical_operators");

        let unless_node = match node.as_unless_node() {
            Some(u) => u,
            None => return,
        };

        let predicate = unless_node.predicate();

        match enforced_style {
            "forbid_logical_operators" => {
                // Flag any logical operators in unless conditions
                if contains_logical_operator(&predicate) {
                    let (line, column) =
                        source.offset_to_line_col(unless_node.keyword_loc().start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use logical operators in `unless` conditions.".to_string(),
                    ));
                }
            }
            "forbid_mixed_logical_operators" | _ => {
                // Flag mixed logical operators (both && and ||)
                if contains_mixed_logical_operators(&predicate) {
                    let (line, column) =
                        source.offset_to_line_col(unless_node.keyword_loc().start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Do not use mixed logical operators in `unless` conditions.".to_string(),
                    ));
                }
            }
        }
    }
}

fn contains_logical_operator(node: &ruby_prism::Node<'_>) -> bool {
    node.as_and_node().is_some() || node.as_or_node().is_some()
}

fn contains_mixed_logical_operators(node: &ruby_prism::Node<'_>) -> bool {
    let mut finder = LogicalOpFinder {
        has_and: false,
        has_or: false,
    };
    finder.visit(node);
    finder.has_and && finder.has_or
}

struct LogicalOpFinder {
    has_and: bool,
    has_or: bool,
}

impl<'pr> Visit<'pr> for LogicalOpFinder {
    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        self.has_and = true;
        ruby_prism::visit_and_node(self, node);
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        self.has_or = true;
        ruby_prism::visit_or_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UnlessLogicalOperators,
        "cops/style/unless_logical_operators"
    );
}
