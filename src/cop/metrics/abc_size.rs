use ruby_prism::Visit;

use crate::cop::node_type::{
    BLOCK_NODE, BLOCK_PARAMETER_NODE, CALL_NODE, CASE_NODE, DEF_NODE, ELSE_NODE, IF_NODE,
    KEYWORD_REST_PARAMETER_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE,
    OPTIONAL_KEYWORD_PARAMETER_NODE, OPTIONAL_PARAMETER_NODE, REQUIRED_KEYWORD_PARAMETER_NODE,
    REQUIRED_PARAMETER_NODE, REST_PARAMETER_NODE, UNLESS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-04)
///
/// Corpus oracle reported FP=269, FN=988.
///
/// FP root cause: nitrocop analyzed `define_method` with dynamic names
/// (`define_method("name_#{suffix}")`), while RuboCop's `MethodComplexity`
/// matcher only handles static `sym`/`str` method names.
///
/// FN root cause: `[]=` setter calls were not counted as assignments.
/// RuboCop's ABC calculator counts setter sends as assignments (and branches).
/// In hash-heavy methods this undercount led to missed offenses.
///
/// ## Corpus investigation (2026-03-07)
///
/// Corpus oracle reported FP=191, FN=551.
///
/// FP root cause #1: CaseNode with else was counting +1 condition, but in
/// RuboCop `case` is NOT in CONDITION_NODES — only `when` nodes are. The
/// `else_branch?` method in RuboCop checks `[:case, :if]` but is only called
/// from `evaluate_condition_node` which requires the node to be in
/// CONDITION_NODES first. Since `case` isn't there, the else bonus is never
/// applied. Fix: removed CaseNode else condition counting entirely.
///
/// FP root cause #2: Score was not rounded to 2 decimal places before the
/// threshold comparison. RuboCop uses `.round(2)` in the calculator, so
/// scores like 17.003 round to 17.0 and don't fire. Fix: round score.
///
/// FN root cause: MultiWriteNode (`a, b, c = expr`) was counted as a single
/// assignment instead of counting each target individually. RuboCop's
/// `compound_assignment` method counts each non-setter child as a separate
/// assignment. Fix: iterate over lefts/rest/rights targets.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=87, FN=368.
///
/// Bug 1: KNOWN_ITERATING_METHODS was a custom list with wrong methods
/// (`times`, `upto`, `downto`, `step`, `loop`, `tap`, `then`, `yield_self`)
/// and missing many correct ones. Replaced with canonical RuboCop list from
/// `vendor/rubocop/lib/rubocop/cop/metrics/utils/iterating_block.rb`.
///
/// Bug 2: `visit_rescue_node` counted +1 condition per rescue clause, but
/// RuboCop has a single `:rescue` node wrapping all `:resbody` clauses.
/// In Prism, rescue clauses chain via `subsequent`, causing over-counting.
/// Fix: use `in_rescue_chain` flag to count only the first rescue clause.
///
/// Bug 3: Inline rescue (`expr rescue fallback`) as `RescueModifierNode`
/// was not counted as a condition. In Parser AST, `:rescue` is in
/// `CONDITION_NODES` which covers both block and inline rescue. Fix: added
/// `RescueModifierNode` to the conditions arm of `count_node()`.
///
/// ## Corpus investigation (2026-03-08, round 2)
///
/// Bug 1 (FN): `CallOrWriteNode` (`obj.foo ||= v`), `CallAndWriteNode`
/// (`obj.foo &&= v`), and `CallOperatorWriteNode` (`obj.foo += v`) were
/// not handled in `count_node`, falling through to `_ => {}`.
/// Fix: added match arms for these node types. `||=`/`&&=` count
/// A+1, B+1, C+1; `+=` etc. count A+1, B+1 (no condition since
/// `op_asgn` is not in `CONDITION_NODES`).
///
/// Bug 2 (FP): Pattern guards in `case/in` (`in :a if guard`) were
/// double-counting. Prism wraps the guard as an `IfNode` inside `InNode`'s
/// pattern, but RuboCop's `if_guard`/`unless_guard` are not in
/// `CONDITION_NODES`. Fix: added `visit_in_node` override with
/// `in_pattern_guard` flag to suppress IfNode/UnlessNode counting inside
/// InNode patterns.
pub struct AbcSize;

/// Known iterating method names that make blocks count toward conditions.
/// Must match RuboCop's `Metrics::Utils::IteratingBlock::KNOWN_ITERATING_METHODS`
/// (enumerable + enumerator + array + hash sets from iterating_block.rb).
const KNOWN_ITERATING_METHODS: &[&[u8]] = &[
    // Enumerable
    b"all?",
    b"any?",
    b"chain",
    b"chunk",
    b"chunk_while",
    b"collect",
    b"collect_concat",
    b"count",
    b"cycle",
    b"detect",
    b"drop",
    b"drop_while",
    b"each",
    b"each_cons",
    b"each_entry",
    b"each_slice",
    b"each_with_index",
    b"each_with_object",
    b"entries",
    b"filter",
    b"filter_map",
    b"find",
    b"find_all",
    b"find_index",
    b"flat_map",
    b"grep",
    b"grep_v",
    b"group_by",
    b"inject",
    b"lazy",
    b"map",
    b"max",
    b"max_by",
    b"min",
    b"min_by",
    b"minmax",
    b"minmax_by",
    b"none?",
    b"one?",
    b"partition",
    b"reduce",
    b"reject",
    b"reverse_each",
    b"select",
    b"slice_after",
    b"slice_before",
    b"slice_when",
    b"sort",
    b"sort_by",
    b"sum",
    b"take",
    b"take_while",
    b"tally",
    b"to_h",
    b"uniq",
    b"zip",
    // Enumerator
    b"with_index",
    b"with_object",
    // Array
    b"bsearch",
    b"bsearch_index",
    b"collect!",
    b"combination",
    b"d_permutation",
    b"delete_if",
    b"each_index",
    b"keep_if",
    b"map!",
    b"permutation",
    b"product",
    b"reject!",
    b"repeat",
    b"repeated_combination",
    b"select!",
    b"sort!",
    b"sort_by",
    // Hash
    b"each_key",
    b"each_pair",
    b"each_value",
    b"fetch",
    b"fetch_values",
    b"has_key?",
    b"merge",
    b"merge!",
    b"transform_keys",
    b"transform_keys!",
    b"transform_values",
    b"transform_values!",
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
    /// Tracks whether we are inside a rescue chain to avoid counting
    /// subsequent rescue clauses (Prism chains them via `subsequent`).
    in_rescue_chain: bool,
    /// Set when visiting an InNode's pattern to suppress counting guard
    /// IfNode/UnlessNode as separate conditions (matching RuboCop where
    /// if_guard/unless_guard are not in CONDITION_NODES).
    in_pattern_guard: bool,
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
            in_rescue_chain: false,
            in_pattern_guard: false,
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
            | ruby_prism::Node::LocalVariableOperatorWriteNode { .. }
            | ruby_prism::Node::InstanceVariableOperatorWriteNode { .. }
            | ruby_prism::Node::ClassVariableOperatorWriteNode { .. }
            | ruby_prism::Node::GlobalVariableOperatorWriteNode { .. }
            | ruby_prism::Node::ConstantOperatorWriteNode { .. }
            | ruby_prism::Node::ConstantPathOperatorWriteNode { .. } => {
                self.assignments += 1;
            }

            // Multi-assignment: `a, b, c = expr` — each target counts as a separate
            // assignment in RuboCop (compound_assignment counts non-setter children).
            // The child LocalVariableTargetNode/etc are not counted elsewhere, so we
            // count them here. Targets starting with _ are excluded.
            ruby_prism::Node::MultiWriteNode { .. } => {
                if let Some(mw) = node.as_multi_write_node() {
                    for target in mw.lefts().iter() {
                        let skip = match &target {
                            ruby_prism::Node::LocalVariableTargetNode { .. } => target
                                .as_local_variable_target_node()
                                .is_some_and(|t| t.name().as_slice().starts_with(b"_")),
                            ruby_prism::Node::SplatNode { .. } => {
                                // Splat targets like `*rest` — check the inner expression
                                if let Some(splat) = target.as_splat_node() {
                                    splat.expression().is_none()
                                        || splat.expression().is_some_and(|expr| {
                                            expr.as_local_variable_target_node().is_some_and(|t| {
                                                t.name().as_slice().starts_with(b"_")
                                            })
                                        })
                                } else {
                                    false
                                }
                            }
                            _ => false,
                        };
                        if !skip {
                            self.assignments += 1;
                        }
                    }
                    // Also count the rest target if present
                    if let Some(rest) = mw.rest() {
                        let skip = if let Some(splat) = rest.as_splat_node() {
                            splat.expression().is_none()
                                || splat.expression().is_some_and(|expr| {
                                    expr.as_local_variable_target_node()
                                        .is_some_and(|t| t.name().as_slice().starts_with(b"_"))
                                })
                        } else {
                            false
                        };
                        if !skip {
                            self.assignments += 1;
                        }
                    }
                    // Count rights too (e.g., `a, *b, c = ...`)
                    for target in mw.rights().iter() {
                        let skip = match &target {
                            ruby_prism::Node::LocalVariableTargetNode { .. } => target
                                .as_local_variable_target_node()
                                .is_some_and(|t| t.name().as_slice().starts_with(b"_")),
                            _ => false,
                        };
                        if !skip {
                            self.assignments += 1;
                        }
                    }
                }
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

            // Call compound assignments: obj.foo ||= v, obj.foo &&= v
            // In Parser AST these produce (or_asgn (send obj :foo) v) — the send counts
            // as a branch, compound_assignment counts as assignment, and or_asgn/and_asgn
            // is in CONDITION_NODES.
            ruby_prism::Node::CallOrWriteNode { .. }
            | ruby_prism::Node::CallAndWriteNode { .. } => {
                self.assignments += 1;
                self.branches += 1;
                self.conditions += 1;
            }
            // Call operator assignment: obj.foo += v
            // In Parser AST: (op_asgn (send obj :foo) :+ v) — send is branch,
            // compound_assignment counts assignment, but op_asgn is NOT in CONDITION_NODES.
            ruby_prism::Node::CallOperatorWriteNode { .. } => {
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
                    if param
                        .name()
                        .is_some_and(|n| !n.as_slice().starts_with(b"_"))
                    {
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
                    if param
                        .name()
                        .is_some_and(|n| !n.as_slice().starts_with(b"_"))
                    {
                        self.assignments += 1;
                    }
                }
            }
            ruby_prism::Node::BlockParameterNode { .. } => {
                if let Some(param) = node.as_block_parameter_node() {
                    if param
                        .name()
                        .is_some_and(|n| !n.as_slice().starts_with(b"_"))
                    {
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
                        if call.call_operator_loc().is_some_and(|loc| {
                            let bytes = loc.as_slice();
                            bytes == b"&."
                        }) && !self.discount_repeated_csend(&call)
                        {
                            self.conditions += 1;
                        }
                        // Iterating block: a call with a block (BlockNode or BlockArgumentNode)
                        // to a known iterating method counts as a condition.
                        // BlockArgumentNode handles `items.map(&:foo)` (block_pass).
                        if call.block().is_some_and(|b| {
                            b.as_block_node().is_some() || b.as_block_argument_node().is_some()
                        }) && KNOWN_ITERATING_METHODS.contains(&method_name)
                        {
                            self.conditions += 1;
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
            // Skip when in_pattern_guard — Prism wraps `in :x if guard` as
            // InNode(pattern=IfNode), and RuboCop's if_guard/unless_guard are not
            // in CONDITION_NODES, so the guard should not count separately.
            ruby_prism::Node::IfNode { .. } => {
                if !self.in_pattern_guard {
                    self.conditions += 1;
                    if let Some(if_node) = node.as_if_node() {
                        // Add +1 for explicit else (not elsif), but NOT for ternary
                        let is_ternary = if_node.if_keyword_loc().is_none();
                        if !is_ternary
                            && if_node
                                .subsequent()
                                .is_some_and(|s| s.as_else_node().is_some())
                        {
                            self.conditions += 1;
                        }
                    }
                }
            }
            // unless is a separate node type in Prism (not an IfNode)
            ruby_prism::Node::UnlessNode { .. } => {
                if !self.in_pattern_guard {
                    self.conditions += 1;
                    if let Some(unless_node) = node.as_unless_node() {
                        if unless_node.else_clause().is_some() {
                            self.conditions += 1;
                        }
                    }
                }
            }
            // CaseNode: `case` is NOT in RuboCop's CONDITION_NODES, so it
            // does not count as a condition at all. Each `when` branch counts
            // as a condition (handled by WhenNode below), but the `else` does
            // NOT add an extra condition — unlike `if/unless` where `else` does.
            // (RuboCop's else_branch? filters for :case/:if types but case is
            //  never reached since it's not in CONDITION_NODES.)
            // ForNode counts as BOTH a condition and an assignment (for the loop variable)
            ruby_prism::Node::ForNode { .. } => {
                self.conditions += 1;
                self.assignments += 1;
            }
            ruby_prism::Node::WhileNode { .. }
            | ruby_prism::Node::UntilNode { .. }
            | ruby_prism::Node::WhenNode { .. }
            | ruby_prism::Node::AndNode { .. }
            | ruby_prism::Node::OrNode { .. }
            | ruby_prism::Node::RescueModifierNode { .. } => {
                self.conditions += 1;
            }
            // InNode is handled in visit_in_node to manage guard suppression.
            // Note: RescueNode is NOT counted here — it is handled in visit_rescue_node
            // to ensure it counts as a single condition regardless of how many
            // rescue clauses exist (Prism chains them via `subsequent`).
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
        // RuboCop counts `rescue` as a single condition for the entire chain.
        // In Prism, rescue clauses are chained via `subsequent`, so visit_rescue_node
        // is called once per clause. Only count +1 for the first rescue in the chain.
        if !self.in_rescue_chain {
            self.conditions += 1;
            self.in_rescue_chain = true;
            ruby_prism::visit_rescue_node(self, node);
            self.in_rescue_chain = false;
        } else {
            ruby_prism::visit_rescue_node(self, node);
        }
    }

    // InNode: count +1 condition for the `in` clause, then visit children with
    // guard suppression. In Prism, `in :x if guard` wraps the pattern as IfNode
    // inside InNode, which would be double-counted without suppression.
    fn visit_in_node(&mut self, node: &ruby_prism::InNode<'pr>) {
        self.conditions += 1;
        // Visit the pattern with guard suppression active so that any
        // IfNode/UnlessNode guard is not counted as a separate condition.
        self.in_pattern_guard = true;
        let pattern = node.pattern();
        self.visit(&pattern);
        self.in_pattern_guard = false;
        // Visit the body normally
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
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
    matches!(name, b"==" | b"===" | b"!=" | b"<=" | b">=" | b">" | b"<")
}

/// Setter methods end in '=' but are not operators (!=, ==, <=, >=).
/// Examples: foo=, bar=
/// In RuboCop, setter method calls count as both a branch and an assignment.
fn is_setter_method(name: &[u8]) -> bool {
    name.len() >= 2
        && name.ends_with(b"=")
        && !matches!(name, b"==" | b"!=" | b"<=" | b">=" | b"===")
}

impl AbcSize {
    fn check_def(
        &self,
        source: &SourceFile,
        def_node: &ruby_prism::DefNode<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let max = config.get_usize("Max", 17);
        let count_repeated_attributes = config.get_bool("CountRepeatedAttributes", true);

        let method_name_str = std::str::from_utf8(def_node.name().as_slice()).unwrap_or("");
        if self.is_allowed_method(method_name_str, config) {
            return;
        }

        // RuboCop's AbcSize passes only the method body to AbcSizeCalculator,
        // so method-level parameters are NOT counted as assignments. Block
        // parameters inside the body ARE counted because the visitor traverses them.
        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        let mut counter = AbcCounter::new(count_repeated_attributes);
        counter.visit(&body);

        let raw_score = counter.score();
        // RuboCop rounds to 2 decimal places before the threshold comparison.
        let score = (raw_score * 100.0).round() / 100.0;
        if score > max as f64 {
            let start_offset = def_node.def_keyword_loc().start_offset();
            let (line, column) = source.offset_to_line_col(start_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Assignment Branch Condition size for {method_name_str} is too high. [{score:.2}/{max}]"
                ),
            ));
        }
    }

    fn check_define_method(
        &self,
        source: &SourceFile,
        call_node: &ruby_prism::CallNode<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Only handle bare define_method calls (no receiver)
        if call_node.name().as_slice() != b"define_method" {
            return;
        }
        if call_node.receiver().is_some() {
            return;
        }

        // Must have a block
        let block = match call_node.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        let max = config.get_usize("Max", 17);
        let count_repeated_attributes = config.get_bool("CountRepeatedAttributes", true);

        // Extract method name from first argument
        let method_name = match extract_define_method_name(call_node) {
            Some(name) => name,
            // RuboCop ignores define_method with dynamic/non-literal names.
            None => return,
        };
        let method_name_str = method_name.as_str();
        if self.is_allowed_method(method_name_str, config) {
            return;
        }

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let mut counter = AbcCounter::new(count_repeated_attributes);
        counter.visit(&body);

        let raw_score = counter.score();
        // RuboCop rounds to 2 decimal places before the threshold comparison.
        let score = (raw_score * 100.0).round() / 100.0;
        if score > max as f64 {
            let start_offset = call_node.location().start_offset();
            let (line, column) = source.offset_to_line_col(start_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Assignment Branch Condition size for {method_name_str} is too high. [{score:.2}/{max}]"
                ),
            ));
        }
    }

    fn is_allowed_method(&self, method_name: &str, config: &CopConfig) -> bool {
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        if let Some(allowed) = &allowed_methods {
            if allowed.iter().any(|m| m == method_name) {
                return true;
            }
        }
        if let Some(patterns) = &allowed_patterns {
            if patterns.iter().any(|p| {
                regex::Regex::new(p)
                    .ok()
                    .is_some_and(|re| re.is_match(method_name))
            }) {
                return true;
            }
        }
        false
    }
}

/// Extract the method name from a `define_method` call's first argument.
/// Handles symbol literals (:name), string literals ("name"), and returns
/// None for dynamic/interpolated names.
fn extract_define_method_name(call: &ruby_prism::CallNode<'_>) -> Option<String> {
    let args = call.arguments()?;
    let first = args.arguments().iter().next()?;

    if let Some(sym) = first.as_symbol_node() {
        return Some(String::from_utf8_lossy(sym.unescaped()).into_owned());
    }
    if let Some(s) = first.as_string_node() {
        return Some(String::from_utf8_lossy(s.unescaped()).into_owned());
    }
    None
}

impl Cop for AbcSize {
    fn name(&self) -> &'static str {
        "Metrics/AbcSize"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            BLOCK_PARAMETER_NODE,
            CALL_NODE,
            CASE_NODE,
            DEF_NODE,
            ELSE_NODE,
            IF_NODE,
            KEYWORD_REST_PARAMETER_NODE,
            LOCAL_VARIABLE_READ_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            OPTIONAL_KEYWORD_PARAMETER_NODE,
            OPTIONAL_PARAMETER_NODE,
            REQUIRED_KEYWORD_PARAMETER_NODE,
            REQUIRED_PARAMETER_NODE,
            REST_PARAMETER_NODE,
            UNLESS_NODE,
        ]
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
        // Handle both `def` nodes and `define_method(:name) do...end` blocks.
        if let Some(def_node) = node.as_def_node() {
            self.check_def(source, &def_node, config, diagnostics);
        } else if let Some(call_node) = node.as_call_node() {
            self.check_define_method(source, &call_node, config, diagnostics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AbcSize, "cops/metrics/abc_size");

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // Multiple assignments and calls push ABC well above 1
        let source = b"def foo\n  a = 1\n  b = 2\n  c = bar\n  d = baz\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire with Max:1 on method with high ABC"
        );
        assert!(diags[0].message.contains("/1]"));
    }

    #[test]
    fn config_count_repeated_attributes_false() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // model is called 3 times; with CountRepeatedAttributes:false it counts as 1 branch
        let source = b"def search\n  x = model\n  y = model\n  z = model\nend\n";

        // With CountRepeatedAttributes:true (default), branches = 3
        let config_true = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                (
                    "CountRepeatedAttributes".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let diags_true = run_cop_full_with_config(&AbcSize, source, config_true);

        // With CountRepeatedAttributes:false, branches = 1
        let config_false = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(3.into())),
                (
                    "CountRepeatedAttributes".into(),
                    serde_yml::Value::Bool(false),
                ),
            ]),
            ..CopConfig::default()
        };
        let _diags_false = run_cop_full_with_config(&AbcSize, source, config_false);

        // ABC with true: A=3, B=3, C=0 => sqrt(9+9) = 4.24 > 3
        assert!(
            !diags_true.is_empty(),
            "Should fire with CountRepeatedAttributes:true"
        );
        // ABC with false: A=3, B=1, C=0 => sqrt(9+1) = 3.16 > 3
        // Actually this still fires. Let me use Max:4 instead
        // A=3, B=1, C=0 => sqrt(9+1) = 3.16 which is > 3 but < 4
        let config_false2 = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(4.into())),
                (
                    "CountRepeatedAttributes".into(),
                    serde_yml::Value::Bool(false),
                ),
            ]),
            ..CopConfig::default()
        };
        let diags_false2 = run_cop_full_with_config(&AbcSize, source, config_false2);
        assert!(
            diags_false2.is_empty(),
            "Should not fire with CountRepeatedAttributes:false and Max:4"
        );

        // Same Max:4 but with true => A=3, B=3, C=0 => 4.24 > 4 => fires
        let config_true2 = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(4.into())),
                (
                    "CountRepeatedAttributes".into(),
                    serde_yml::Value::Bool(true),
                ),
            ]),
            ..CopConfig::default()
        };
        let diags_true2 = run_cop_full_with_config(&AbcSize, source, config_true2);
        assert!(
            !diags_true2.is_empty(),
            "Should fire with CountRepeatedAttributes:true and Max:4"
        );
    }

    #[test]
    fn define_method_offense() {
        use crate::testutil::run_cop_full;

        // 18 assignments in define_method block => ABC = 18 > 17
        let source = b"define_method(:complex_dm) do\n  a = 1\n  b = 2\n  c = 3\n  d = 4\n  e = 5\n  f = 6\n  g = 7\n  h = 8\n  i = 9\n  j = 10\n  k = 11\n  l = 12\n  m = 13\n  n = 14\n  o = 15\n  p = 16\n  q = 17\n  r = 18\nend\n";
        let diags = run_cop_full(&AbcSize, source);
        assert!(
            !diags.is_empty(),
            "Should fire on define_method with high ABC"
        );
        assert!(
            diags[0].message.contains("complex_dm"),
            "Message should include method name"
        );
    }

    #[test]
    fn define_method_no_offense() {
        use crate::testutil::run_cop_full;

        let source = b"define_method(:simple) do\n  x = 1\n  x\nend\n";
        let diags = run_cop_full(&AbcSize, source);
        assert!(diags.is_empty(), "Should not fire on short define_method");
    }

    #[test]
    fn define_method_string_name() {
        use crate::testutil::run_cop_full;

        // define_method with string arg
        let source = b"define_method(\"complex_dm\") do\n  a = 1\n  b = 2\n  c = 3\n  d = 4\n  e = 5\n  f = 6\n  g = 7\n  h = 8\n  i = 9\n  j = 10\n  k = 11\n  l = 12\n  m = 13\n  n = 14\n  o = 15\n  p = 16\n  q = 17\n  r = 18\nend\n";
        let diags = run_cop_full(&AbcSize, source);
        assert!(
            !diags.is_empty(),
            "Should fire on define_method with string name"
        );
        assert!(
            diags[0].message.contains("complex_dm"),
            "Message should include string method name"
        );
    }

    #[test]
    fn block_pass_iterating_method() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // items.map(&:foo) — with block_pass, map should count as condition
        // A=1 (x), B=2 (map, transform), C=1 (map block_pass condition)
        // With Max:1, the score sqrt(1+4+1) = 2.45 > 1
        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"def foo\n  x = items.map(&:bar)\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(
            !diags.is_empty(),
            "Should count block_pass iterating method as condition"
        );
    }

    /// Bug 1: `times` is NOT in RuboCop's KNOWN_ITERATING_METHODS.
    /// 5.times { ... } should NOT count as an iterating block condition.
    /// A=1 (x), B=2 (times, puts), C=0 => sqrt(1+4) = 2.24
    /// With Max:1, this fires. But if times wrongly adds C+1, score = sqrt(1+4+1) = 2.45.
    /// The key check: conditions should be 0, not 1.
    #[test]
    fn times_not_iterating_method() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // 5.times { puts "hi" } — times should NOT count as iterating
        // A=0, B=2 (times, puts), C=0 => sqrt(0+4+0) = 2.0
        // If times wrongly counted: C=1 => sqrt(0+4+1) = 2.24
        let source = b"def foo\n  5.times { puts \"hi\" }\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config.clone());
        assert!(!diags.is_empty(), "Should fire with Max:1");
        // Score should be [2.00/1] not [2.24/1]
        assert!(
            diags[0].message.contains("[2.00/1]"),
            "times should NOT count as iterating condition, got: {}",
            diags[0].message
        );
    }

    /// Bug 2: Multiple rescue clauses should only count +1 condition total,
    /// not +1 per clause. RuboCop has one :rescue node wrapping all :resbody clauses.
    #[test]
    fn rescue_chain_counts_once() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // A=1 (x), B=1 (dangerous), C=1 (rescue chain = 1 condition)
        // Score = sqrt(1+1+1) = 1.73
        // If rescue over-counts: C=3 => sqrt(1+1+9) = 3.32
        let source = b"def foo\n  x = dangerous\nrescue ArgumentError\n  nil\nrescue TypeError\n  nil\nrescue StandardError\n  nil\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:1");
        assert!(
            diags[0].message.contains("[1.73/1]"),
            "Rescue chain should count as 1 condition, not 3. Got: {}",
            diags[0].message
        );
    }

    /// Bug 3: Inline rescue (`x rescue nil`) should count as +1 condition.
    /// RuboCop's Parser AST has :rescue in CONDITION_NODES for this too.
    #[test]
    fn inline_rescue_counts_as_condition() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // x = foo rescue nil
        // A=1 (x), B=1 (foo), C=1 (rescue modifier)
        // Score = sqrt(1+1+1) = 1.73
        // Without rescue counting: C=0 => sqrt(1+1+0) = 1.41
        let source = b"def bar\n  x = foo rescue nil\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:1");
        assert!(
            diags[0].message.contains("[1.73/1]"),
            "Inline rescue should count as a condition. Got: {}",
            diags[0].message
        );
    }

    /// CallOrWriteNode (obj.foo ||= v): CallOrWriteNode adds A+1, B+1, C+1.
    /// `obj` is also a receiverless CallNode adding B+1.
    /// Total: A=1, B=2, C=1 => sqrt(1+4+1) = 2.45
    #[test]
    fn call_or_write_node_counts() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"def test_method\n  obj.foo ||= 1\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire for obj.foo ||= v");
        assert!(
            diags[0].message.contains("[2.45/1]"),
            "obj.foo ||= v should be A=1,B=2,C=1 => 2.45. Got: {}",
            diags[0].message
        );
    }

    /// CallAndWriteNode (obj.bar &&= v): same as CallOrWriteNode.
    /// A=1, B=2 (obj call + bar setter), C=1 => sqrt(1+4+1) = 2.45
    #[test]
    fn call_and_write_node_counts() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"def test_method\n  obj.bar &&= 1\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire for obj.bar &&= v");
        assert!(
            diags[0].message.contains("[2.45/1]"),
            "obj.bar &&= v should be A=1,B=2,C=1 => 2.45. Got: {}",
            diags[0].message
        );
    }

    /// CallOperatorWriteNode (obj.count += v): A+1, B+1 from node, plus B+1
    /// from `obj` receiverless call. No condition.
    /// Total: A=1, B=2, C=0 => sqrt(1+4) = 2.24
    #[test]
    fn call_operator_write_node_counts() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"def test_method\n  obj.count += 1\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire for obj.count += v");
        assert!(
            diags[0].message.contains("[2.24/1]"),
            "obj.count += v should be A=1,B=2,C=0 => 2.24. Got: {}",
            diags[0].message
        );
    }

    /// Pattern guard in case/in should not double-count IfNode conditions.
    /// `case x; in :a if guard` — `in` counts C+1, guard IfNode should NOT.
    /// `x` is a receiverless call (B+1).
    /// Total: A=1 (y), B=1 (x), C=1 (in) => sqrt(1+1+1) = 1.73
    #[test]
    fn case_in_pattern_guard_no_double_count() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"def test_method\n  case x\n  in :a if true\n    y = 1\n  end\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:1");
        assert!(
            diags[0].message.contains("[1.73/1]"),
            "Pattern guard should not add extra condition. Got: {}",
            diags[0].message
        );
    }

    #[test]
    fn allowed_patterns_regex() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        // AllowedPatterns with regex pattern ^_
        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(1.into())),
                (
                    "AllowedPatterns".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("^_".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // Method starting with underscore should be allowed by regex
        let source = b"def _internal\n  a = 1\n  b = foo\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(
            diags.is_empty(),
            "Should skip method matching AllowedPatterns regex ^_"
        );
    }
}
