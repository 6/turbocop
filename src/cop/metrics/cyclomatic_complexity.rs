use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CyclomaticComplexity;

#[derive(Default)]
struct CyclomaticCounter {
    complexity: usize,
    /// Tracks whether we are already inside a rescue chain to avoid
    /// counting subsequent rescue clauses (Prism chains them via `subsequent`).
    in_rescue_chain: bool,
}

/// Known iterating method names that make blocks count toward complexity.
const KNOWN_ITERATING_METHODS: &[&[u8]] = &[
    b"each", b"each_with_index", b"each_with_object", b"each_pair",
    b"each_key", b"each_value", b"each_slice", b"each_cons",
    b"each_line", b"each_byte", b"each_char", b"each_codepoint",
    b"map", b"flat_map", b"collect", b"collect_concat",
    b"select", b"filter", b"find_all", b"reject", b"filter_map",
    b"detect", b"find", b"find_index", b"rindex",
    b"reduce", b"inject", b"any?", b"all?", b"none?", b"one?",
    b"count", b"sum", b"min", b"max", b"min_by", b"max_by",
    b"minmax", b"minmax_by", b"sort_by", b"group_by",
    b"partition", b"zip", b"take_while", b"drop_while",
    b"chunk", b"chunk_while", b"slice_before", b"slice_after", b"slice_when",
    b"times", b"upto", b"downto", b"step",
    b"loop", b"tap", b"then", b"yield_self",
    b"each_index", b"reverse_each",
];

impl CyclomaticCounter {
    fn count_node(&mut self, node: &ruby_prism::Node<'_>) {
        match node {
            ruby_prism::Node::IfNode { .. }
            | ruby_prism::Node::UnlessNode { .. }
            | ruby_prism::Node::WhileNode { .. }
            | ruby_prism::Node::UntilNode { .. }
            | ruby_prism::Node::ForNode { .. }
            | ruby_prism::Node::WhenNode { .. }
            | ruby_prism::Node::AndNode { .. }
            | ruby_prism::Node::OrNode { .. }
            | ruby_prism::Node::InNode { .. } => {
                self.complexity += 1;
            }
            // Note: RescueNode is NOT counted here — it is handled in visit_rescue_node
            // to ensure it counts as a single decision point regardless of how many
            // rescue clauses exist (Prism chains them via `subsequent`).

            // or_asgn (||=) and and_asgn (&&=) count as conditions
            ruby_prism::Node::LocalVariableOrWriteNode { .. }
            | ruby_prism::Node::InstanceVariableOrWriteNode { .. }
            | ruby_prism::Node::ClassVariableOrWriteNode { .. }
            | ruby_prism::Node::GlobalVariableOrWriteNode { .. }
            | ruby_prism::Node::ConstantOrWriteNode { .. }
            | ruby_prism::Node::ConstantPathOrWriteNode { .. }
            | ruby_prism::Node::LocalVariableAndWriteNode { .. }
            | ruby_prism::Node::InstanceVariableAndWriteNode { .. }
            | ruby_prism::Node::ClassVariableAndWriteNode { .. }
            | ruby_prism::Node::GlobalVariableAndWriteNode { .. }
            | ruby_prism::Node::ConstantAndWriteNode { .. }
            | ruby_prism::Node::ConstantPathAndWriteNode { .. } => {
                self.complexity += 1;
            }

            // CallNode: count &. (safe navigation) and iterating blocks
            ruby_prism::Node::CallNode { .. } => {
                if let Some(call) = node.as_call_node() {
                    // Safe navigation (&.) counts
                    if call.call_operator_loc().is_some_and(|loc| loc.as_slice() == b"&.") {
                        self.complexity += 1;
                    }
                    // Iterating block counts
                    if call.block().is_some_and(|b| b.as_block_node().is_some()) {
                        let method_name = call.name().as_slice();
                        if KNOWN_ITERATING_METHODS.contains(&method_name) {
                            self.complexity += 1;
                        }
                    }
                }
            }

            _ => {}
        }
    }
}

impl<'pr> Visit<'pr> for CyclomaticCounter {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.count_node(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.count_node(&node);
    }

    // RescueNode is visited via visit_rescue_node (not visit_branch_node_enter)
    // because Prism's visit_begin_node calls visitor.visit_rescue_node directly.
    // In Prism, rescue clauses are chained via `subsequent`, so visit_rescue_node
    // is called once per clause. RuboCop counts `rescue` as a single decision point
    // (one `rescue` node in the Parser AST wraps all clauses), so we only count +1
    // for the first rescue in the chain.
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        if !self.in_rescue_chain {
            self.complexity += 1;
            self.in_rescue_chain = true;
            ruby_prism::visit_rescue_node(self, node);
            self.in_rescue_chain = false;
        } else {
            ruby_prism::visit_rescue_node(self, node);
        }
    }
}

impl Cop for CyclomaticComplexity {
    fn name(&self) -> &'static str {
        "Metrics/CyclomaticComplexity"
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

        let max = config.get_usize("Max", 7);

        // AllowedMethods / AllowedPatterns: skip methods matching these
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let method_name_str =
            std::str::from_utf8(def_node.name().as_slice()).unwrap_or("");
        if let Some(allowed) = &allowed_methods {
            if allowed.iter().any(|m| m == method_name_str) {
                return Vec::new();
            }
        }
        if let Some(patterns) = &allowed_patterns {
            if patterns.iter().any(|p| method_name_str.contains(p.as_str())) {
                return Vec::new();
            }
        }

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut counter = CyclomaticCounter::default();
        counter.visit(&body);

        let score = 1 + counter.complexity;
        if score > max {
            let method_name =
                std::str::from_utf8(def_node.name().as_slice()).unwrap_or("unknown");
            let start_offset = def_node.def_keyword_loc().start_offset();
            let (line, column) = source.offset_to_line_col(start_offset);
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Cyclomatic complexity for {method_name} is too high. [{score}/{max}]"
                ),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CyclomaticComplexity, "cops/metrics/cyclomatic_complexity");

    #[test]
    fn config_custom_max() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // 1 (base) + 1 (if) = 2 > Max:1
        let source = b"def foo\n  if x\n    y\n  end\nend\n";
        let diags = run_cop_full_with_config(&CyclomaticComplexity, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:1 on method with if branch");
        assert!(diags[0].message.contains("[2/1]"));
    }
}
