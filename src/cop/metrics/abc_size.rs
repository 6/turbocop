use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct AbcSize;

#[derive(Default)]
struct AbcCounter {
    assignments: usize,
    branches: usize,
    conditions: usize,
}

impl AbcCounter {
    fn count_node(&mut self, node: &ruby_prism::Node<'_>) {
        match node {
            // A (Assignments)
            ruby_prism::Node::LocalVariableWriteNode { .. }
            | ruby_prism::Node::InstanceVariableWriteNode { .. }
            | ruby_prism::Node::ClassVariableWriteNode { .. }
            | ruby_prism::Node::GlobalVariableWriteNode { .. }
            | ruby_prism::Node::ConstantWriteNode { .. }
            | ruby_prism::Node::ConstantPathWriteNode { .. }
            | ruby_prism::Node::MultiWriteNode { .. }
            | ruby_prism::Node::LocalVariableOperatorWriteNode { .. }
            | ruby_prism::Node::InstanceVariableOperatorWriteNode { .. }
            | ruby_prism::Node::ClassVariableOperatorWriteNode { .. }
            | ruby_prism::Node::GlobalVariableOperatorWriteNode { .. }
            | ruby_prism::Node::ConstantOperatorWriteNode { .. }
            | ruby_prism::Node::ConstantPathOperatorWriteNode { .. }
            | ruby_prism::Node::LocalVariableAndWriteNode { .. }
            | ruby_prism::Node::LocalVariableOrWriteNode { .. }
            | ruby_prism::Node::InstanceVariableAndWriteNode { .. }
            | ruby_prism::Node::InstanceVariableOrWriteNode { .. }
            | ruby_prism::Node::ClassVariableAndWriteNode { .. }
            | ruby_prism::Node::ClassVariableOrWriteNode { .. }
            | ruby_prism::Node::GlobalVariableAndWriteNode { .. }
            | ruby_prism::Node::GlobalVariableOrWriteNode { .. }
            | ruby_prism::Node::ConstantAndWriteNode { .. }
            | ruby_prism::Node::ConstantOrWriteNode { .. }
            | ruby_prism::Node::ConstantPathAndWriteNode { .. }
            | ruby_prism::Node::ConstantPathOrWriteNode { .. } => {
                self.assignments += 1;
            }

            // B (Branches)
            ruby_prism::Node::CallNode { .. } => {
                self.branches += 1;
            }

            // C (Conditions)
            ruby_prism::Node::IfNode { .. }
            | ruby_prism::Node::WhileNode { .. }
            | ruby_prism::Node::UntilNode { .. }
            | ruby_prism::Node::ForNode { .. }
            | ruby_prism::Node::WhenNode { .. }
            | ruby_prism::Node::RescueNode { .. }
            | ruby_prism::Node::AndNode { .. }
            | ruby_prism::Node::OrNode { .. } => {
                self.conditions += 1;
            }

            _ => {}
        }
    }

    fn score(&self) -> f64 {
        let a = self.assignments as f64;
        let b = self.branches as f64;
        let c = self.conditions as f64;
        (a * a + b * b + c * c).sqrt()
    }
}

impl<'pr> Visit<'pr> for AbcCounter {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.count_node(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.count_node(&node);
    }
}

impl Cop for AbcSize {
    fn name(&self) -> &'static str {
        "Metrics/AbcSize"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let max = config
            .options
            .get("Max")
            .and_then(|v| v.as_u64())
            .unwrap_or(17) as usize;

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut counter = AbcCounter::default();
        counter.visit(&body);

        let score = counter.score();
        if score > max as f64 {
            let method_name =
                std::str::from_utf8(def_node.name().as_slice()).unwrap_or("unknown");
            let start_offset = def_node.def_keyword_loc().start_offset();
            let (line, column) = source.offset_to_line_col(start_offset);
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: format!(
                    "Assignment Branch Condition size for {method_name} is too high. [{score:.2}/{max}]"
                ),
            }];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &AbcSize,
            include_bytes!("../../../testdata/cops/metrics/abc_size/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &AbcSize,
            include_bytes!("../../../testdata/cops/metrics/abc_size/no_offense.rb"),
        );
    }
}
