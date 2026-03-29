use crate::cop::node_type::{AND_NODE, OR_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Matches RuboCop's mixed-logical-operator checks for `unless` predicates.
///
/// Fixed FNs where Prism hid nested `&&`/`||` inside wrapper nodes that the old
/// recursion skipped, including assignment values, unary `!` calls, method
/// arguments, and attached blocks. We still preserve RuboCop's top-level paren
/// behavior by only treating the predicate as direct `and`/`or` when the root
/// node itself is an `AndNode` or `OrNode`.
pub struct UnlessLogicalOperators;

impl Cop for UnlessLogicalOperators {
    fn name(&self) -> &'static str {
        "Style/UnlessLogicalOperators"
    }

    fn default_enabled(&self) -> bool {
        false
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
            _ => {
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

/// Check if the condition has mixed logical operators.
fn contains_mixed_logical_operators(node: &ruby_prism::Node<'_>) -> bool {
    let mut collector = LogicalOperatorCollector::default();
    collector.visit(node);

    // Keep RuboCop's root-node behavior: outer parentheses around the entire
    // condition do not make it a direct `and`/`or` predicate.
    (node.as_or_node().is_some() && collector.has_and())
        || (node.as_and_node().is_some() && collector.has_or())
        || collector.mixed_and()
        || collector.mixed_or()
}

#[derive(Default)]
struct LogicalOperatorCollector {
    and_symbolic: usize,
    and_keyword: usize,
    or_symbolic: usize,
    or_keyword: usize,
}

impl LogicalOperatorCollector {
    fn has_and(&self) -> bool {
        self.and_symbolic + self.and_keyword > 0
    }

    fn has_or(&self) -> bool {
        self.or_symbolic + self.or_keyword > 0
    }

    fn mixed_and(&self) -> bool {
        self.and_symbolic > 0 && self.and_keyword > 0
    }

    fn mixed_or(&self) -> bool {
        self.or_symbolic > 0 && self.or_keyword > 0
    }
}

impl<'pr> Visit<'pr> for LogicalOperatorCollector {
    fn visit_and_node(&mut self, node: &ruby_prism::AndNode<'pr>) {
        if node.operator_loc().as_slice() == b"&&" {
            self.and_symbolic += 1;
        } else {
            self.and_keyword += 1;
        }
        ruby_prism::visit_and_node(self, node);
    }

    fn visit_or_node(&mut self, node: &ruby_prism::OrNode<'pr>) {
        if node.operator_loc().as_slice() == b"||" {
            self.or_symbolic += 1;
        } else {
            self.or_keyword += 1;
        }
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
