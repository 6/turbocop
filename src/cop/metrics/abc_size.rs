use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETER_NODE, CALL_NODE, CASE_NODE, DEF_NODE, ELSE_NODE, IF_NODE, KEYWORD_REST_PARAMETER_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE, REQUIRED_PARAMETER_NODE, REST_PARAMETER_NODE, UNLESS_NODE};

pub struct AbcSize;

/// Known iterating method names that make blocks count toward conditions.
/// Matches the list in CyclomaticComplexity/PerceivedComplexity.
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

struct AbcCounter {
    assignments: usize,
    branches: usize,
    conditions: usize,
    count_repeated_attributes: bool,
    seen_attributes: std::collections::HashSet<Vec<u8>>,
    /// Tracks local variable names that have been seen with `&.` (safe navigation).
    /// RuboCop discounts repeated `&.` on the same variable — only the first counts
    /// as a condition. When the variable is reassigned, it is removed from the set.
    seen_csend_vars: std::collections::HashSet<Vec<u8>>,
}

impl AbcCounter {
    fn new(count_repeated_attributes: bool) -> Self {
        Self {
            assignments: 0,
            branches: 0,
            conditions: 0,
            count_repeated_attributes,
            seen_attributes: std::collections::HashSet::new(),
            seen_csend_vars: std::collections::HashSet::new(),
        }
    }

    /// Check if a &. call on a local variable is a repeat (discount it).
    /// Returns true if this csend should be discounted (i.e., it's a repeat).
    fn discount_repeated_csend(&mut self, call: &ruby_prism::CallNode<'_>) -> bool {
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return false,
        };
        let lvar = match receiver.as_local_variable_read_node() {
            Some(l) => l,
            None => return false,
        };
        let var_name = lvar.name().as_slice().to_vec();
        // Insert returns false if the value was already present (= repeated)
        !self.seen_csend_vars.insert(var_name)
    }

    fn count_node(&mut self, node: &ruby_prism::Node<'_>) {
        match node {
            // A (Assignments) — variable writes, op-assigns
            // Note: underscore-prefixed locals (_foo = ...) are NOT counted
            ruby_prism::Node::LocalVariableWriteNode { .. } => {
                if let Some(lvar) = node.as_local_variable_write_node() {
                    let name = lvar.name().as_slice();
                    // Reset csend tracking for this variable on reassignment
                    self.seen_csend_vars.remove(name);
                    if !name.starts_with(b"_") {
                        self.assignments += 1;
                    }
                }
            }
            ruby_prism::Node::InstanceVariableWriteNode { .. }
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
            | ruby_prism::Node::ConstantPathOperatorWriteNode { .. } => {
                self.assignments += 1;
            }

            // ||= and &&= count as BOTH assignment AND condition in RuboCop.
            // In the Parser gem, `x ||= v` has a nested lvasgn child that counts
            // as an assignment. In Prism these are single nodes, so we count both here.
            ruby_prism::Node::LocalVariableAndWriteNode { .. }
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
                self.conditions += 1;
            }

            // Index compound assignments: hash["key"] ||= v, hash["key"] &&= v, hash["key"] += v
            // In the Parser gem these are (or_asgn (send :[] ...) v) — the send child counts as
            // a branch, and compound_assignment counts a non-setter send child as an assignment.
            // The ||=/&&= also counts as a condition (or_asgn/and_asgn in CONDITION_NODES).
            ruby_prism::Node::IndexOrWriteNode { .. }
            | ruby_prism::Node::IndexAndWriteNode { .. } => {
                // A: assignment from the indexed write
                // B: implicit [] call (receiver lookup)
                // C: the ||=/&&= conditional
                self.assignments += 1;
                self.branches += 1;
                self.conditions += 1;
            }
            ruby_prism::Node::IndexOperatorWriteNode { .. } => {
                // A: assignment from the indexed write
                // B: implicit [] call (receiver lookup)
                // (no condition — op_asgn is not in CONDITION_NODES)
                self.assignments += 1;
                self.branches += 1;
            }

            // Method/block parameters count as assignments in RuboCop (argument_type? nodes).
            // Only counted when the name doesn't start with underscore.
            ruby_prism::Node::RequiredParameterNode { .. } => {
                if let Some(param) = node.as_required_parameter_node() {
                    if !param.name().as_slice().starts_with(b"_") {
                        self.assignments += 1;
                    }
                }
            }
            ruby_prism::Node::OptionalParameterNode { .. } => {
                if let Some(param) = node.as_optional_parameter_node() {
                    if !param.name().as_slice().starts_with(b"_") {
                        self.assignments += 1;
                    }
                }
            }
            ruby_prism::Node::RestParameterNode { .. } => {
                if let Some(param) = node.as_rest_parameter_node() {
                    if param.name().is_some_and(|n| !n.as_slice().starts_with(b"_")) {
                        self.assignments += 1;
                    }
                }
            }
            ruby_prism::Node::RequiredKeywordParameterNode { .. } => {
                if let Some(param) = node.as_required_keyword_parameter_node() {
                    if !param.name().as_slice().starts_with(b"_") {
                        self.assignments += 1;
                    }
                }
            }
            ruby_prism::Node::OptionalKeywordParameterNode { .. } => {
                if let Some(param) = node.as_optional_keyword_parameter_node() {
                    if !param.name().as_slice().starts_with(b"_") {
                        self.assignments += 1;
                    }
                }
            }
            ruby_prism::Node::KeywordRestParameterNode { .. } => {
                if let Some(param) = node.as_keyword_rest_parameter_node() {
                    if param.name().is_some_and(|n| !n.as_slice().starts_with(b"_")) {
                        self.assignments += 1;
                    }
                }
            }
            ruby_prism::Node::BlockParameterNode { .. } => {
                if let Some(param) = node.as_block_parameter_node() {
                    if param.name().is_some_and(|n| !n.as_slice().starts_with(b"_")) {
                        self.assignments += 1;
                    }
                }
            }

            // B (Branches) — send/csend/yield
            // Comparison methods (==, !=, <, >, <=, >=, ===) count as conditions,
            // not branches, matching RuboCop's behavior.
            // Setter methods (name ending in =) count as BOTH assignment AND branch.
            ruby_prism::Node::CallNode { .. } => {
                if let Some(call) = node.as_call_node() {
                    let method_name = call.name().as_slice();
                    if is_comparison_method(method_name) {
                        // Comparison operators are conditions, not branches
                        self.conditions += 1;
                    } else {
                        if !self.count_repeated_attributes {
                            // An "attribute" is a receiverless call with no arguments
                            let has_no_args = call.arguments().is_none();
                            let is_receiverless = call.receiver().is_none();
                            if has_no_args && is_receiverless {
                                let name = method_name.to_vec();
                                if !self.seen_attributes.insert(name) {
                                    // Already seen this attribute, don't count again
                                    return;
                                }
                            }
                        }
                        // Setter methods (self.foo = v, obj.bar = v) count as assignment too
                        if is_setter_method(method_name) {
                            self.assignments += 1;
                        }
                        self.branches += 1;
                        // Safe navigation (&.) adds an extra condition, matching
                        // RuboCop where csend is both a branch and a condition.
                        // But repeated &. on the same local variable is discounted.
                        if call.call_operator_loc().map_or(false, |loc| {
                                let bytes = loc.as_slice();
                                bytes == b"&."
                            })
                        {
                            if !self.discount_repeated_csend(&call) {
                                self.conditions += 1;
                            }
                        }
                        // Iterating block: a call with a block to a known iterating method
                        // counts as a condition (block is in RuboCop's CONDITION_NODES).
                        if call.block().is_some_and(|b| b.as_block_node().is_some()) {
                            if KNOWN_ITERATING_METHODS.contains(&method_name) {
                                self.conditions += 1;
                            }
                        }
                    }
                }
            }

            // yield counts as a branch
            ruby_prism::Node::YieldNode { .. } => {
                self.branches += 1;
            }

            // C (Conditions)
            // if/unless/case with explicit 'else' gets +2 (one for the condition, one for else)
            // Ternary (x ? y : z) has no if_keyword_loc and counts as 1 (not 2).
            ruby_prism::Node::IfNode { .. } => {
                self.conditions += 1;
                if let Some(if_node) = node.as_if_node() {
                    // Add +1 for explicit else (not elsif), but NOT for ternary
                    let is_ternary = if_node.if_keyword_loc().is_none();
                    if !is_ternary && if_node.subsequent().is_some_and(|s| s.as_else_node().is_some()) {
                        self.conditions += 1;
                    }
                }
            }
            // unless is a separate node type in Prism (not an IfNode)
            ruby_prism::Node::UnlessNode { .. } => {
                self.conditions += 1;
                if let Some(unless_node) = node.as_unless_node() {
                    if unless_node.else_clause().is_some() {
                        self.conditions += 1;
                    }
                }
            }
            ruby_prism::Node::CaseNode { .. } => {
                // case itself is not in CONDITION_NODES but we check for else
                if let Some(case_node) = node.as_case_node() {
                    if case_node.else_clause().is_some() {
                        self.conditions += 1;
                    }
                }
            }
            // ForNode counts as BOTH a condition and an assignment (for the loop variable)
            ruby_prism::Node::ForNode { .. } => {
                self.conditions += 1;
                self.assignments += 1;
            }
            ruby_prism::Node::WhileNode { .. }
            | ruby_prism::Node::UntilNode { .. }
            | ruby_prism::Node::WhenNode { .. }
            | ruby_prism::Node::RescueNode { .. }
            | ruby_prism::Node::AndNode { .. }
            | ruby_prism::Node::OrNode { .. }
            | ruby_prism::Node::InNode { .. } => {
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

    // The Prism visitor calls specific visit_*_node methods for certain child nodes,
    // bypassing visit_branch_node_enter/visit_leaf_node_enter. We need to override
    // these to ensure our counter sees all relevant nodes.

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        // Count rescue as a condition
        self.conditions += 1;
        // Delegate to default implementation to recurse into children
        ruby_prism::visit_rescue_node(self, node);
    }

    fn visit_else_node(&mut self, node: &ruby_prism::ElseNode<'pr>) {
        // ElseNode itself doesn't directly add to counts — the parent IfNode/CaseNode
        // handles else counting. Just delegate to visit children.
        ruby_prism::visit_else_node(self, node);
    }

    fn visit_ensure_node(&mut self, node: &ruby_prism::EnsureNode<'pr>) {
        ruby_prism::visit_ensure_node(self, node);
    }
}

/// RuboCop comparison operators: ==, ===, !=, <=, >=, >, <
/// These are counted as conditions, not branches, in ABC metric.
fn is_comparison_method(name: &[u8]) -> bool {
    matches!(
        name,
        b"==" | b"===" | b"!=" | b"<=" | b">=" | b">" | b"<"
    )
}

/// Setter methods end in '=' but are not operators (!=, ==, <=, >=, []=).
/// Examples: foo=, bar=
/// In RuboCop, setter method calls count as both a branch and an assignment.
fn is_setter_method(name: &[u8]) -> bool {
    name.len() >= 2
        && name.ends_with(b"=")
        && !matches!(name, b"==" | b"!=" | b"<=" | b">=" | b"===" | b"[]=")
}

impl Cop for AbcSize {
    fn name(&self) -> &'static str {
        "Metrics/AbcSize"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETER_NODE, CALL_NODE, CASE_NODE, DEF_NODE, ELSE_NODE, IF_NODE, KEYWORD_REST_PARAMETER_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE, OPTIONAL_KEYWORD_PARAMETER_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE, REQUIRED_PARAMETER_NODE, REST_PARAMETER_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        let max = config.get_usize("Max", 17);
        let count_repeated_attributes = config.get_bool("CountRepeatedAttributes", true);

        // AllowedMethods / AllowedPatterns: skip methods matching these
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let method_name_str =
            std::str::from_utf8(def_node.name().as_slice()).unwrap_or("");
        if let Some(allowed) = &allowed_methods {
            if allowed.iter().any(|m| m == method_name_str) {
                return;
            }
        }
        if let Some(patterns) = &allowed_patterns {
            if patterns.iter().any(|p| method_name_str.contains(p.as_str())) {
                return;
            }
        }

        let mut counter = AbcCounter::new(count_repeated_attributes);

        // RuboCop's AbcSize passes only the method body to AbcSizeCalculator,
        // so method-level parameters are NOT counted as assignments. Block
        // parameters inside the body ARE counted because the visitor traverses them.
        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };
        counter.visit(&body);

        let score = counter.score();
        if score > max as f64 {
            let method_name =
                std::str::from_utf8(def_node.name().as_slice()).unwrap_or("unknown");
            let start_offset = def_node.def_keyword_loc().start_offset();
            let (line, column) = source.offset_to_line_col(start_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Assignment Branch Condition size for {method_name} is too high. [{score:.2}/{max}]"
                ),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AbcSize, "cops/metrics/abc_size");

    #[test]
    fn config_custom_max() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // Multiple assignments and calls push ABC well above 1
        let source = b"def foo\n  a = 1\n  b = 2\n  c = bar\n  d = baz\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:1 on method with high ABC");
        assert!(diags[0].message.contains("/1]"));
    }

    #[test]
    fn config_count_repeated_attributes_false() {
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        // model is called 3 times; with CountRepeatedAttributes:false it counts as 1 branch
        let source = b"def search\n  x = model\n  y = model\n  z = model\nend\n";

        // With CountRepeatedAttributes:true (default), branches = 3
        let config_true = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                ("CountRepeatedAttributes".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags_true = run_cop_full_with_config(&AbcSize, source, config_true);

        // With CountRepeatedAttributes:false, branches = 1
        let config_false = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                ("CountRepeatedAttributes".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let _diags_false = run_cop_full_with_config(&AbcSize, source, config_false);

        // ABC with true: A=3, B=3, C=0 => sqrt(9+9) = 4.24 > 3
        assert!(!diags_true.is_empty(), "Should fire with CountRepeatedAttributes:true");
        // ABC with false: A=3, B=1, C=0 => sqrt(9+1) = 3.16 > 3
        // Actually this still fires. Let me use Max:4 instead
        // A=3, B=1, C=0 => sqrt(9+1) = 3.16 which is > 3 but < 4
        let config_false2 = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(4.into())),
                ("CountRepeatedAttributes".into(), serde_yml::Value::Bool(false)),
            ]),
            ..CopConfig::default()
        };
        let diags_false2 = run_cop_full_with_config(&AbcSize, source, config_false2);
        assert!(diags_false2.is_empty(), "Should not fire with CountRepeatedAttributes:false and Max:4");

        // Same Max:4 but with true => A=3, B=3, C=0 => 4.24 > 4 => fires
        let config_true2 = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(4.into())),
                ("CountRepeatedAttributes".into(), serde_yml::Value::Bool(true)),
            ]),
            ..CopConfig::default()
        };
        let diags_true2 = run_cop_full_with_config(&AbcSize, source, config_true2);
        assert!(!diags_true2.is_empty(), "Should fire with CountRepeatedAttributes:true and Max:4");
    }
}
