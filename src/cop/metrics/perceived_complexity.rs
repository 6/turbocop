use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct PerceivedComplexity;

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
    b"each_index", b"reverse_each",
];

#[derive(Default)]
struct PerceivedCounter {
    complexity: usize,
    /// Tracks whether we are already inside a rescue chain to avoid
    /// counting subsequent rescue clauses (Prism chains them via `subsequent`).
    in_rescue_chain: bool,
}

impl PerceivedCounter {
    fn count_node(&mut self, node: &ruby_prism::Node<'_>) {
        match node {
            // if with else (not elsif) counts as 2, otherwise 1
            // Ternary (x ? y : z) has no if_keyword_loc and counts as 1 (not 2).
            ruby_prism::Node::IfNode { .. } => {
                if let Some(if_node) = node.as_if_node() {
                    let is_ternary = if_node.if_keyword_loc().is_none();
                    if !is_ternary && if_node.subsequent().is_some_and(|s| s.as_else_node().is_some()) {
                        self.complexity += 2;
                    } else {
                        self.complexity += 1;
                    }
                }
            }
            // unless is a separate node type in Prism
            ruby_prism::Node::UnlessNode { .. } => {
                if let Some(unless_node) = node.as_unless_node() {
                    if unless_node.else_clause().is_some() {
                        self.complexity += 2;
                    } else {
                        self.complexity += 1;
                    }
                }
            }

            ruby_prism::Node::WhileNode { .. }
            | ruby_prism::Node::UntilNode { .. }
            | ruby_prism::Node::ForNode { .. }
            | ruby_prism::Node::AndNode { .. }
            | ruby_prism::Node::OrNode { .. }
            | ruby_prism::Node::InNode { .. } => {
                self.complexity += 1;
            }
            // Note: RescueNode is NOT counted here — it is handled in visit_rescue_node
            // to ensure it counts as a single decision point regardless of how many
            // rescue clauses exist (Prism chains them via `subsequent`).

            // case with condition: 0.8 + 0.2 * branches (rounded)
            // case without condition (case/when with no predicate): when nodes count individually
            ruby_prism::Node::CaseNode { .. } => {
                if let Some(case_node) = node.as_case_node() {
                    let nb_whens = case_node.conditions().iter().count();
                    let has_else = case_node.else_clause().is_some();
                    let nb_branches = nb_whens + if has_else { 1 } else { 0 };

                    if case_node.predicate().is_some() {
                        // case expr; when ... -> 0.8 + 0.2 * branches
                        self.complexity += ((nb_branches as f64 * 0.2) + 0.8).round() as usize;
                    } else {
                        // case; when ... -> each when counts
                        self.complexity += nb_branches;
                    }
                }
            }

            // case/in (pattern matching) - similar to case/when
            ruby_prism::Node::CaseMatchNode { .. } => {
                if let Some(case_match) = node.as_case_match_node() {
                    let nb_ins = case_match.conditions().iter().count();
                    let has_else = case_match.else_clause().is_some();
                    let nb_branches = nb_ins + if has_else { 1 } else { 0 };

                    if case_match.predicate().is_some() {
                        self.complexity += ((nb_branches as f64 * 0.2) + 0.8).round() as usize;
                    } else {
                        self.complexity += nb_branches;
                    }
                }
            }

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

            // Note: ElseNode is NOT counted separately in PerceivedComplexity.
            // Instead, if+else counts as 2 (handled above in IfNode).
            // WhenNode is NOT counted either - case handles the scoring.
            _ => {}
        }
    }
}

impl<'pr> Visit<'pr> for PerceivedCounter {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.count_node(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.count_node(&node);
    }

    // RescueNode is visited via visit_rescue_node (not visit_branch_node_enter)
    // because Prism's visit_begin_node calls visitor.visit_rescue_node directly.
    // In Prism, rescue clauses are chained via `subsequent`, so visit_rescue_node
    // is called once per clause. RuboCop counts `rescue` as a single decision point,
    // so we only count +1 for the first rescue in the chain.
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

impl Cop for PerceivedComplexity {
    fn name(&self) -> &'static str {
        "Metrics/PerceivedComplexity"
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

        let max = config.get_usize("Max", 8);

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

        let mut counter = PerceivedCounter::default();
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
                    "Perceived complexity for {method_name} is too high. [{score}/{max}]"
                ),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PerceivedComplexity, "cops/metrics/perceived_complexity");

    #[test]
    fn config_custom_max() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // 1 (base) + 2 (if with else) = 3 > Max:1
        let source = b"def foo\n  if x\n    y\n  else\n    z\n  end\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:1 on method with if/else");
        assert!(diags[0].message.contains("/1]"));
    }
}
