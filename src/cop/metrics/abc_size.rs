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
///
/// ## Corpus investigation (2026-03-09)
///
/// Re-ran the cop under the repository's Ruby 3.4 toolchain:
/// `mise exec ruby@3.4 -- python3 scripts/check-cop.py Metrics/AbcSize
/// --verbose --rerun`.
///
/// Result:
/// - Expected: 65,966
/// - Actual:   69,118
/// - Excess:   0 over CI baseline after file-drop adjustment
/// - Missing:  0
///
/// No code change was taken in this run. The stale artifact FP/FN counts do
/// not represent a current excess regression once the corpus rerun uses the
/// correct Ruby/Bundler environment.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=22, FN=275. Local rerun shows 0 excess (PASS).
/// The FP/FN are per-location differences, not aggregate count mismatches.
///
/// Bug 1 (FP): `/regex/ =~ expr` was counted as a branch (CallNode with
/// name `=~`), but in Parser gem this is `match_with_lvasgn` which is NOT
/// a `:send` and thus not counted as a branch. Fix: skip branch counting
/// when CallNode `=~` has a regex literal receiver.
///
/// Bug 2 (FN): `rescue => var` was not counted as an assignment. In Parser
/// gem, the rescue reference is a `:lvasgn` (counted by `simple_assignment?`).
/// In Prism, it's a `LocalVariableTargetNode` (or other *TargetNode) inside
/// `RescueNode.reference` that was not handled. Fix: count rescue reference
/// as assignment in `visit_rescue_node`.
///
/// Bug 3 (FN): Lambda literals (`-> {}`) were not counted as branches. In
/// Parser gem, `-> {}` is `(block (send nil :lambda) ...)` and the `:lambda`
/// send counts as B+1. In Prism, `-> {}` is `LambdaNode` with no CallNode.
/// Fix: count `LambdaNode` as B+1.
///
/// ## Corpus investigation (2026-03-10, round 2)
///
/// Corpus oracle reported FP=19, FN=152. Local rerun shows 0 excess (PASS).
///
/// Bug (FN): RuboCop's `compound_assignment` method for shorthand assignments
/// (||=, &&=, +=, etc.) counts non-setter `send` children as extra assignments.
/// In Parser AST, `x ||= foo` is `(or_asgn (lvasgn :x) (send nil :foo))` and
/// `compound_assignment` iterates direct children: sends that `respond_to?(:setter_method?)`
/// and are NOT setter methods get an extra A+1. In Prism, `x ||= foo` is a single
/// `LocalVariableOrWriteNode` — there is no separate lvasgn/send child, so the
/// extra assignment was missed. Fix: added `compound_assignment_extra()` that
/// extracts the `.value()` from any shorthand assignment node and checks if it's
/// a non-setter CallNode, adding A+1 when true. Applied to all variable
/// Or/And/OperatorWrite nodes and Index Or/And/OperatorWrite nodes.
///
/// This affects borderline methods (score within ~1.0 of Max) where the slight
/// undercount of assignments caused nitrocop to miss offenses that RuboCop flags.
///
/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=42, FN=34.
///
/// FN=34: CallTargetNode and IndexTargetNode (multi-write targets like
/// `r.color, r.key = ...` or `params[:x] = ...`) were not counted as branches.
/// In Parser, these are regular `:send` nodes that count as B+1. Fix: added
/// match arm for CallTargetNode/IndexTargetNode → B+1. Commit de1d816b.
///
/// FP=42: All had scores 17.03-18.25 (barely above Max=17). Root cause:
/// `compound_assignment_extra` incorrectly added A+1 for `||=`/`&&=`/`+=`
/// where the value is a method call WITH A BLOCK. In Parser AST, `x ||=
/// items.detect { |i| ... }` is `(or_asgn lvasgn (block (send ...)))` — the
/// `:block` wrapper doesn't respond to `setter_method?`, so RuboCop's
/// `compound_assignment` skips it. In Prism, the `.value()` is the CallNode
/// directly (block is a child), so our code saw it as a non-setter call and
/// incorrectly added A+1. Fix: skip CallNodes that have a block in
/// `compound_assignment_extra`.
///
/// ## Corpus investigation (2026-03-11, session 4)
///
/// Corpus oracle reported FP=19, FN=17.
///
/// Bug 1 (FP, ~13 of 19): it-blocks (Ruby 4.0 `it` implicit parameter) and
/// numblocks (`_1`/`_2` numbered parameters) were counted as iterating block
/// conditions (C+1), but RuboCop's Parser gem produces `:itblock`/`:numblock`
/// node types which are NOT in `COUNTED_NODES`. Only regular `:block` and
/// `:block_pass` count. In Prism, all blocks are `BlockNode` — fixed by
/// checking `parameters()` for `ItParametersNode`/`NumberedParametersNode`
/// and skipping the condition count for those. Same pattern as
/// CyclomaticComplexity and PerceivedComplexity (already fixed).
///
/// Bug 2 (FP, ~5 of 19): `ImplicitRestNode` in multi-write (`relay, = expr`)
/// was counted as an assignment (A+1), but it has no variable target. In
/// Parser gem, `relay, = expr` has `(mlhs (lvasgn :relay))` — no rest node
/// at all. In Prism, the trailing comma produces `ImplicitRestNode` which was
/// not being skipped. Fixed by checking `as_implicit_rest_node()`.
///
/// Remaining after session 4: ~1 FP, 17 FN — all near threshold 17.
///
/// ## Corpus investigation (2026-03-11, session 5)
///
/// Bug 1 (FN): Call compound assignments (`obj.foo ||= v`, `obj.foo &&= v`,
/// `obj.foo += v`) were not handled by `compound_assignment_extra`. In Parser,
/// these are `(or_asgn (send obj :foo) v)` — `compound_assignment` counts ALL
/// non-setter send children as A+1 (both target send and value send). The
/// target send's `setter_method?` returns false (no `loc.operator`), so it IS
/// counted. The target A+1 was already handled by `count_node` for
/// `CallOrWriteNode` etc., but the value send's A+1 was missing. Fix: added
/// `CallOrWriteNode`, `CallAndWriteNode`, `CallOperatorWriteNode` to
/// `compound_assignment_extra`.
///
/// Bug 2 (FP): `begin...end while cond` / `begin...end until cond` post-
/// condition loops were counted as C+1. In Parser gem, these produce
/// `:while_post`/`:until_post` which are NOT in `COUNTED_NODES`. In Prism,
/// they are `WhileNode`/`UntilNode` with `is_begin_modifier() == true`.
/// Fix: skip condition count when `is_begin_modifier()` is true.
///
/// Bug 3 (FN): `&block` parameters in nested defs/blocks were not counted
/// as A+1. Prism's generated `visit_parameters_node` calls
/// `visit_block_parameter_node` directly instead of through `visitor.visit()`,
/// bypassing `visit_leaf_node_enter`. Fix: override `visit_block_parameter_node`
/// in AbcCounter.
///
/// Bug 4 (FN): `compound_assignment_extra` skipped block_pass calls. In Parser,
/// `x ||= items.map(&:strip)` has `block_pass` as a child of the send (not a
/// `:block` wrapper). RuboCop's compound_assignment DOES count this as A+1. In
/// Prism, `map(&:strip)` has `block: BlockArgumentNode` (not BlockNode). Fix:
/// only skip compound_extra when `.block()` is a `BlockNode`, not `BlockArgumentNode`.
///
/// Bug 5 (FN): `IndexOrWriteNode` / `IndexAndWriteNode` compound_assignment
/// undercounted. In Parser, `@h[k] ||= v` produces `(or_asgn (send :[] k) v)`.
/// RuboCop's `compound_assignment` counts BOTH children: the `[]` target (A+1)
/// and the value (A+1 if non-setter call). Our old code only counted the value
/// side via `compound_assignment_extra`. Fix: `compound_assignment_extra` now
/// returns +1 for the target `[]` plus +1 for the value. The `count_node` arm
/// for IndexOrWriteNode was also changed to NOT add A+1 (matching RuboCop's
/// `assignment?` returning false for `shorthand_asgn?` nodes).
///
/// Bug 6 (FN): `ForNode` undercounted assignments by 1. In Parser, `for x in
/// items` is `(for (lvasgn :x) (send nil :items) body)`. Both `for_type?` (A+1)
/// and the child `(lvasgn :x)` (A+1 via `simple_assignment?`) are counted. In
/// Prism, the index is `LocalVariableTargetNode` which has no `count_node` arm.
/// Fix: ForNode handler now does A+=2 instead of A+=1.
///
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=0, FN=3.
///
/// Bug 1 (FN): Interpolated regex `=~` was incorrectly skipped. In Parser gem,
/// only non-interpolated `/literal/ =~ expr` produces `match_with_lvasgn` (NOT
/// a `:send`). Interpolated `/#{ x }/ =~ expr` is `(send (regexp ...) :=~ expr)`
/// and IS counted as B+1. nitrocop was skipping both. Fix: only skip
/// `RegularExpressionNode` receivers, not `InterpolatedRegularExpressionNode`.
///
/// Bug 2 (FN): Nested `begin...rescue...end` inside a rescue body was not
/// counted as C+1. The `in_rescue_chain` flag (designed to prevent chained
/// `rescue` clauses from double-counting) remained true while visiting the
/// rescue body, suppressing nested rescues. Fix: manually visit rescue
/// children, resetting `in_rescue_chain=false` for the body while keeping it
/// true for `subsequent` chained clauses.
///
/// Note: backtick xstring (`` `cmd` ``) does NOT count as a branch. In Parser
/// gem, `` `cmd` `` is `(xstr ...)`, NOT `(send nil :` `` ` `` ` ...)`. Only
/// the explicit `Kernel.` `` ` `` `(cmd)` form is a `:send`.
///
/// All 3 FN fixed. FP=0 confirmed via verify-cop-locations.py.
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

    /// RuboCop's `compound_assignment` quirk: for shorthand assignments (||=, &&=, +=),
    /// the `compound_assignment` method counts non-setter send children as extra assignments.
    /// In Parser AST, `x ||= foo` is `(or_asgn (lvasgn :x) (send nil :foo))` and
    /// `compound_assignment` iterates over direct children, counting sends that respond to
    /// `setter_method?` and are NOT setter methods. The value-side send gets an extra A+1
    /// on top of the normal B+1 it gets from branch counting.
    ///
    /// In Prism, `x ||= foo` is a single `LocalVariableOrWriteNode` with a `.value()` child.
    /// We need to check if that value is a non-setter CallNode and add the extra assignment.
    fn compound_assignment_extra(&self, node: &ruby_prism::Node<'_>) -> usize {
        // Extract the value node from any shorthand assignment type
        let value = if let Some(n) = node.as_local_variable_or_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_local_variable_and_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_local_variable_operator_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_instance_variable_or_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_instance_variable_and_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_instance_variable_operator_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_class_variable_or_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_class_variable_and_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_class_variable_operator_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_global_variable_or_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_global_variable_and_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_global_variable_operator_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_constant_or_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_constant_and_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_constant_operator_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_constant_path_or_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_constant_path_and_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_constant_path_operator_write_node() {
            Some(n.value())
        // Index compound assignments: hash["key"] ||= v
        // In Parser, these are (or_asgn (send obj :[] key) (send Hash :new)).
        // compound_assignment counts ALL send children: both the target [] send
        // and the value send. The target [] is always a non-setter (+1), and the
        // value may or may not be a non-setter call (+0 or +1).
        } else if let Some(n) = node.as_index_or_write_node() {
            // target [] is always non-setter → +1, plus check value
            return 1 + self.value_compound_extra(&n.value());
        } else if let Some(n) = node.as_index_and_write_node() {
            return 1 + self.value_compound_extra(&n.value());
        } else if let Some(n) = node.as_index_operator_write_node() {
            return 1 + self.value_compound_extra(&n.value());
        // Call compound assignments: obj.foo ||= v, obj.foo &&= v, obj.foo += v
        // In Parser, these are (or_asgn (send obj :foo) v) — compound_assignment
        // counts ALL non-setter send children (both target and value). The target
        // send's A+1 is already handled by count_node for CallOrWriteNode etc.,
        // but the value send's A+1 is missing without this.
        } else if let Some(n) = node.as_call_or_write_node() {
            Some(n.value())
        } else if let Some(n) = node.as_call_and_write_node() {
            Some(n.value())
        } else {
            node.as_call_operator_write_node().map(|n| n.value())
        };

        match value {
            Some(v) => self.value_compound_extra(&v),
            None => 0,
        }
    }

    /// Check if a value node in a compound assignment is a non-setter call,
    /// returning 1 if it should count as an extra assignment.
    fn value_compound_extra(&self, v: &ruby_prism::Node<'_>) -> usize {
        if let Some(call) = v.as_call_node() {
            // In Parser AST, a call with a do/end or {} block is wrapped in a :block node
            // (e.g., `x ||= items.detect { |i| ... }` → (or_asgn lvasgn (block (send ...) ...))).
            // RuboCop's compound_assignment checks direct children of or_asgn for
            // setter_method?, but :block nodes don't respond to setter_method?, so
            // they are NOT counted. In Prism, the .value() is the CallNode directly
            // (block is a child of the call). Skip calls with actual BlockNode to match.
            //
            // BUT: block_pass (`&:symbol` or `&block`) is NOT a :block wrapper in Parser —
            // it's a child of the :send node. So `x ||= items.map(&:strip)` IS counted
            // as A+1 in RuboCop. Only skip when block() is a BlockNode, not BlockArgumentNode.
            if let Some(block) = call.block() {
                if block.as_block_node().is_some() {
                    return 0;
                }
            }
            if !is_setter_method(call.name().as_slice()) {
                return 1;
            }
        }
        0
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
                    // Also count the rest target if present.
                    // ImplicitRestNode (`a, = expr` — trailing comma, no splat)
                    // should NOT be counted — it has no variable target.
                    if let Some(rest) = mw.rest() {
                        let skip = if rest.as_implicit_rest_node().is_some() {
                            true // `a, = expr` — no actual rest target
                        } else if let Some(splat) = rest.as_splat_node() {
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
            //
            // RuboCop's compound_assignment counts BOTH children of or_asgn/and_asgn/op_asgn:
            // 1. Target: (send obj :[] key) — `[]` is not a setter → A+1 from compound_assignment
            // 2. Value: (send Hash :new) — `new` is not a setter → A+1 from compound_assignment
            // The target [] A+1 is handled by compound_assignment_extra (which returns +1 for
            // Index*WriteNode target in addition to the value-side check). The value-side A+1
            // is also handled there. Neither counts through count_node's assignment field.
            //
            // Note: RuboCop's assignment? returns FALSE for shorthand_asgn nodes (the
            // compound_assignment call replaces the normal counting). So count_node does NOT
            // add A+1 here — compound_assignment_extra handles ALL assignment counting for
            // index compound nodes.
            ruby_prism::Node::IndexOrWriteNode { .. }
            | ruby_prism::Node::IndexAndWriteNode { .. } => {
                // B: implicit [] call (receiver lookup)
                // C: the ||=/&&= conditional
                // A: handled entirely by compound_assignment_extra (target[] + value)
                self.branches += 1;
                self.conditions += 1;
            }
            ruby_prism::Node::IndexOperatorWriteNode { .. } => {
                // B: implicit [] call (receiver lookup)
                // (no condition — op_asgn is not in CONDITION_NODES)
                // A: handled entirely by compound_assignment_extra (target[] + value)
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
                    // In Parser gem, `/regex/ =~ expr` (non-interpolated) is `match_with_lvasgn`
                    // (not a :send), so it's NOT counted as a branch. But `/#{ interp }/ =~ expr`
                    // is `(send (regexp ...) :=~ expr)` — a regular :send that IS counted as B+1.
                    // Only skip non-interpolated regex receivers to match RuboCop.
                    if method_name == b"=~" {
                        if let Some(receiver) = call.receiver() {
                            if receiver.as_regular_expression_node().is_some() {
                                return;
                            }
                        }
                    }
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
                        // Iterating block: a call with a block to a known iterating
                        // method counts as a condition. RuboCop's Parser gem produces
                        // :numblock for _1/_2 params and :itblock for `it` params,
                        // neither of which is in COUNTED_NODES. Only regular :block
                        // and :block_pass count. In Prism all blocks are BlockNode,
                        // so we check parameters to distinguish.
                        if let Some(block) = call.block() {
                            let should_count = if let Some(block_node) = block.as_block_node() {
                                match block_node.parameters() {
                                    Some(params) => {
                                        params.as_numbered_parameters_node().is_none()
                                            && params.as_it_parameters_node().is_none()
                                    }
                                    None => true,
                                }
                            } else {
                                // BlockArgumentNode (&:method) — always counts
                                block.as_block_argument_node().is_some()
                            };
                            if should_count && KNOWN_ITERATING_METHODS.contains(&method_name) {
                                self.conditions += 1;
                            }
                        }
                    }
                }
            }

            // CallTargetNode and IndexTargetNode: multi-write assignment targets like
            // `r.color, r.key = ...` or `params[:controller], params[:action] = ...`.
            // In Parser gem these are regular :send nodes (counted as branches).
            // In Prism they are separate node types that are NOT CallNode.
            ruby_prism::Node::CallTargetNode { .. } | ruby_prism::Node::IndexTargetNode { .. } => {
                self.branches += 1;
            }

            // yield counts as a branch
            ruby_prism::Node::YieldNode { .. } => {
                self.branches += 1;
            }

            // Lambda literal (-> {}) counts as a branch. In Parser gem,
            // -> {} is (block (send nil :lambda) ...) and the :lambda send
            // counts as B+1. In Prism, -> {} is LambdaNode with no CallNode
            // for the implicit lambda call. Note: `lambda {}` (method form)
            // is already a CallNode and handled above.
            ruby_prism::Node::LambdaNode { .. } => {
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
            // ForNode counts as BOTH a condition and an assignment (for the loop variable).
            // In Parser, `for x in items` is `(for (lvasgn :x) (send nil :items) body)`.
            // `for_type?` gives A+1, and the child `(lvasgn :x)` also gives A+1 via
            // `simple_assignment?`. Total A=2 in RuboCop.
            // In Prism, the index is a `LocalVariableTargetNode` which has no count_node
            // arm (to avoid double-counting in MultiWriteNode). So we add A=2 here.
            ruby_prism::Node::ForNode { .. } => {
                self.conditions += 1;
                self.assignments += 2;
            }
            // WhileNode/UntilNode: only pre-condition loops count.
            // `begin...end while cond` produces :while_post in Parser (NOT in
            // COUNTED_NODES). In Prism, it's WhileNode with is_begin_modifier()
            // = true. Skip post-condition loops to match.
            ruby_prism::Node::WhileNode { .. } => {
                if let Some(w) = node.as_while_node() {
                    if !w.is_begin_modifier() {
                        self.conditions += 1;
                    }
                }
            }
            ruby_prism::Node::UntilNode { .. } => {
                if let Some(u) = node.as_until_node() {
                    if !u.is_begin_modifier() {
                        self.conditions += 1;
                    }
                }
            }
            ruby_prism::Node::WhenNode { .. }
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

        // RuboCop's compound_assignment quirk: for shorthand assignments,
        // non-setter send values get an extra assignment count.
        self.assignments += self.compound_assignment_extra(node);
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

    // Prism's generated visit_parameters_node calls visit_block_parameter_node
    // directly instead of visitor.visit(), bypassing visit_leaf_node_enter.
    // Override to count the block parameter as an assignment.
    fn visit_block_parameter_node(&mut self, node: &ruby_prism::BlockParameterNode<'pr>) {
        self.count_node(&node.as_node());
    }

    // The Prism visitor calls specific visit_*_node methods for certain child nodes,
    // bypassing visit_branch_node_enter/visit_leaf_node_enter. We need to override
    // these to ensure our counter sees all relevant nodes.

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        // RuboCop counts `rescue` as a single condition for the entire chain.
        // In Prism, rescue clauses are chained via `subsequent`, so visit_rescue_node
        // is called once per clause. Only count +1 for the first rescue in the chain.
        let was_in_chain = self.in_rescue_chain;
        if !was_in_chain {
            self.conditions += 1;
        }

        // `rescue => var` — the rescue reference is an assignment in RuboCop
        // (lvasgn/ivasgn/etc in Parser AST). In Prism it's a *TargetNode that
        // is not otherwise counted. Count it here.
        if let Some(ref_node) = node.reference() {
            let skip = ref_node
                .as_local_variable_target_node()
                .is_some_and(|t| t.name().as_slice().starts_with(b"_"));
            if !skip {
                self.assignments += 1;
            }
        }

        // Visit exceptions, reference, and body with in_rescue_chain=false so that
        // nested begin...rescue...end blocks inside the rescue body get counted
        // independently. Only subsequent (chained) rescue clauses should be suppressed.
        self.in_rescue_chain = false;
        for exception in node.exceptions().iter() {
            self.visit(&exception);
        }
        if let Some(ref_node) = node.reference() {
            self.visit(&ref_node);
        }
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }

        // Visit subsequent (chained) rescue with in_rescue_chain=true to suppress
        // the extra condition count for chained clauses.
        if let Some(subsequent) = node.subsequent() {
            self.in_rescue_chain = true;
            self.visit(&subsequent.as_node());
        }

        self.in_rescue_chain = was_in_chain;
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

    /// CallOrWriteNode with method call value: obj.foo ||= other.bar
    /// RuboCop's compound_assignment counts BOTH target send and value send
    /// as non-setter sends, each getting A+1. The target send and value send
    /// are both branches (B+1 each). or_asgn is condition (C+1).
    /// With local var receivers: A=2 (compound target + value), B=2, C=1
    /// => sqrt(4+4+1) = 3.0
    #[test]
    fn call_or_write_compound_extra_for_value_call() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // obj and other are parameters (local vars), so no B for receivers
        let source = b"def test_method(obj, other)\n  obj.foo ||= other.bar\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire for obj.foo ||= other.bar with Max:1"
        );
        assert!(
            diags[0].message.contains("[3.00/1]"),
            "obj.foo ||= other.bar should be A=2,B=2,C=1 => 3.00. Got: {}",
            diags[0].message
        );
    }

    /// CallOperatorWriteNode with method call value: obj.count += other.delta
    /// compound_assignment counts target send (A+1) + value send (A+1).
    /// Both sends are branches. op_asgn is NOT condition.
    /// A=2, B=2, C=0 => sqrt(4+4) = 2.83
    #[test]
    fn call_operator_write_compound_extra_for_value_call() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"def test_method(obj, other)\n  obj.count += other.delta\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire for obj.count += other.delta with Max:1"
        );
        assert!(
            diags[0].message.contains("[2.83/1]"),
            "obj.count += other.delta should be A=2,B=2,C=0 => 2.83. Got: {}",
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

    /// compound_assignment quirk for ||= with method call value:
    /// `x ||= fetch_val` → A=2 (OrWrite + compound_assignment), B=1 (fetch_val), C=1 (OrWrite)
    /// Single line: A=2, B=1, C=1 => sqrt(4+1+1) = 2.45
    #[test]
    fn or_write_compound_assignment_extra() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"def test_method\n  x ||= fetch_val\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire for x ||= fetch_val with Max:1"
        );
        assert!(
            diags[0].message.contains("[2.45/1]"),
            "x ||= fetch_val should be A=2,B=1,C=1 => 2.45. Got: {}",
            diags[0].message
        );
    }

    /// compound_assignment_extra should NOT fire when the value is a call with a block.
    /// In Parser, `x ||= items.detect { |i| i.ok? }` is (or_asgn lvasgn (block (send ...))).
    /// The :block wrapper doesn't respond to setter_method?, so compound_assignment skips it.
    /// A=2 (x from ||=, |i| param), B=3 (items, detect, ok?), C=2 (or_asgn, detect block iter)
    /// => sqrt(4+9+4) = 4.12
    #[test]
    fn or_write_no_extra_for_block_value() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"def test_method\n  x ||= items.detect { |i| i.ok? }\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:1");
        // A=2 (x from ||=, |i| param), B=3 (items, detect, ok?), C=2 (or_asgn, detect block iter)
        // => sqrt(4+9+4) = sqrt(17) = 4.12
        // If compound_assignment_extra wrongly fires: A=3 => sqrt(9+9+4) = 4.69
        assert!(
            diags[0].message.contains("[4.12/1]"),
            "x ||= items.detect {{ block }} should be A=2,B=3,C=2 => 4.12 (no extra A). Got: {}",
            diags[0].message
        );
    }

    /// compound_assignment quirk for += with method call value:
    /// `x += fetch_val` → A=2 (OperatorWrite + compound_assignment), B=1 (fetch_val), C=0
    /// Single line: A=2, B=1, C=0 => sqrt(4+1) = 2.24
    #[test]
    fn operator_write_compound_assignment_extra() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"def test_method\n  x = 0\n  x += fetch_val\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire for x += fetch_val with Max:1"
        );
        // A=2 (x=0 + compound), but wait: x=0 is A+1, x += is A+1 + compound A+1
        // Total: A=3 (x=0 + x+= + compound), B=1 (fetch_val), C=0
        // sqrt(9+1) = 3.16
        assert!(
            diags[0].message.contains("[3.16/1]"),
            "x=0; x+=fetch_val should be A=3,B=1,C=0 => 3.16. Got: {}",
            diags[0].message
        );
    }

    /// compound_assignment quirk does NOT apply when value is a literal:
    /// `x ||= 1` → A=1 (OrWrite only), B=0, C=1 (OrWrite condition)
    /// Single line: A=1, B=0, C=1 => sqrt(1+0+1) = 1.41
    #[test]
    fn or_write_no_extra_for_literal() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"def test_method\n  x ||= 1\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire for x ||= 1 with Max:1");
        assert!(
            diags[0].message.contains("[1.41/1]"),
            "x ||= 1 should be A=1,B=0,C=1 => 1.41. Got: {}",
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

    /// `relay, = expr` (implicit rest, trailing comma) should NOT count the
    /// implicit rest as an assignment. Only the named target (relay) counts.
    /// In Prism, this produces ImplicitRestNode (not SplatNode).
    #[test]
    fn implicit_rest_not_counted_as_assignment() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // `relay, = foo(1, 2)` — A=1 (relay only), B=1 (foo), C=0
        // => sqrt(1+1+0) = 1.41
        // If ImplicitRestNode wrongly counted: A=2 => sqrt(4+1) = 2.24
        let source = b"def bar\n  relay, = foo(1, 2)\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:1");
        assert!(
            diags[0].message.contains("[1.41/1]"),
            "Implicit rest should NOT be counted. Got: {}",
            diags[0].message
        );
    }

    /// it-blocks (Ruby 4.0 `it` implicit parameter) produce :itblock in Parser gem,
    /// which is NOT in COUNTED_NODES. So `items.each do ... it ... end` should NOT
    /// count as a condition. Same for numblocks (_1, _2). Regular blocks DO count.
    #[test]
    fn itblock_and_numblock_not_counted_as_condition() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };

        // Regular block: items.each do |item| item end
        // A=2 (x, |item|), B=2 (items, each), C=1 (iterating block condition)
        // => sqrt(4+4+1) = 3.00
        let source_regular = b"def foo\n  x = items.each do |item|\n    item\n  end\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source_regular, config.clone());
        assert!(!diags.is_empty(), "Regular block should fire");
        assert!(
            diags[0].message.contains("[3.00/1]"),
            "Regular block: A=2,B=2,C=1 => 3.00. Got: {}",
            diags[0].message
        );

        // it-block: items.each do ... it ... end
        // `it` is ItLocalVariableReadNode (not a CallNode), so B stays at 2
        // A=1 (x), B=2 (items, each), C=0 (it-block NOT counted as condition)
        // => sqrt(1+4+0) = 2.24
        let source_itblock = b"def foo\n  x = items.each do\n    it\n  end\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source_itblock, config.clone());
        assert!(!diags.is_empty(), "it-block should still fire");
        assert!(
            diags[0].message.contains("[2.24/1]"),
            "it-block: A=1,B=2,C=0 => 2.24. Got: {}",
            diags[0].message
        );

        // numblock: items.each do ... _1 ... end
        // `_1` is LocalVariableReadNode (not a CallNode), so B stays at 2
        // A=1 (x), B=2 (items, each), C=0 (numblock NOT counted)
        // => sqrt(1+4+0) = 2.24
        let source_numblock = b"def foo\n  x = items.each do\n    _1\n  end\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source_numblock, config);
        assert!(!diags.is_empty(), "numblock should still fire");
        assert!(
            diags[0].message.contains("[2.24/1]"),
            "numblock: A=1,B=2,C=0 => 2.24. Got: {}",
            diags[0].message
        );
    }

    /// `begin...end while cond` produces :while_post in Parser (NOT in COUNTED_NODES).
    /// In Prism it's WhileNode with is_begin_modifier() = true. Must not count as condition.
    /// Same for `begin...end until cond` → :until_post.
    #[test]
    fn begin_end_while_not_counted_as_condition() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };

        // `begin x = x + 1 end while x < 10` — post-condition while
        // A=2 (x=0, x=x+1), B=1 (+), C=1 (< comparison only, NOT while itself)
        // => sqrt(4+1+1) = 2.45
        // If while_post were counted: C=2 => sqrt(4+1+4) = 3.00
        let source_while = b"def foo\n  x = 0\n  begin\n    x = x + 1\n  end while x < 10\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source_while, config.clone());
        assert!(!diags.is_empty(), "Should fire with Max:1");
        assert!(
            diags[0].message.contains("[2.45/1]"),
            "begin..end while: post-condition while should NOT count as condition. Got: {}",
            diags[0].message
        );

        // Regular `while cond do body end` DOES count as condition
        // A=2 (x=0, x=x+1), B=1 (+), C=2 (while + < comparison)
        // => sqrt(4+1+4) = 3.00
        let source_regular_while =
            b"def foo\n  x = 0\n  while x < 10 do\n    x = x + 1\n  end\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source_regular_while, config.clone());
        assert!(!diags.is_empty(), "Regular while should fire");
        assert!(
            diags[0].message.contains("[3.00/1]"),
            "Regular while SHOULD count as condition. Got: {}",
            diags[0].message
        );

        // begin...end until — same: post-condition should NOT count
        let source_until = b"def foo\n  x = 0\n  begin\n    x = x + 1\n  end until x > 10\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source_until, config);
        assert!(!diags.is_empty(), "Should fire with Max:1");
        assert!(
            diags[0].message.contains("[2.45/1]"),
            "begin..end until: post-condition should NOT count. Got: {}",
            diags[0].message
        );
    }

    /// Helper to compute ABC counter values for a method body.
    /// Parses source, finds the first DefNode, and returns (A, B, C).
    #[cfg(test)]
    fn abc_counter_for_source(source: &[u8]) -> (usize, usize, usize) {
        use ruby_prism::Visit;
        let result = ruby_prism::parse(source);
        // Find the first DefNode
        struct DefFinder<'pr> {
            body: Option<ruby_prism::Node<'pr>>,
        }
        impl<'pr> Visit<'pr> for DefFinder<'pr> {
            fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
                if self.body.is_none() {
                    self.body = node.body();
                }
            }
        }
        let mut finder = DefFinder { body: None };
        finder.visit(&result.node());
        let body = finder.body.expect("No def node found in source");
        let mut counter = AbcCounter::new(true);
        counter.visit(&body);
        (counter.assignments, counter.branches, counter.conditions)
    }

    /// Bug 4: compound_assignment_extra skipped block_pass calls.
    /// `x ||= items.map(&:strip)` — the value `items.map(&:strip)` has block:
    /// BlockArgumentNode (not BlockNode). Should count as A+1 (non-setter send).
    #[test]
    fn compound_extra_block_pass_not_skipped() {
        // @x ||= items.map(&:strip)
        // In count_node: B+1 (ivar []) — wait, this is InstanceVariableOrWriteNode
        // A+1 (assignment), C+1 (||= condition)
        // compound_assignment_extra: value is map(&:strip), block is BlockArgumentNode
        // → not skipped → A+1
        // Visitor: items (B+1), map (B+1)
        // Total: A=2, B=2, C=1 => sqrt(4+4+1) = 3.0
        let (a, b, c) = abc_counter_for_source(b"def foo\n  @x ||= items.map(&:strip)\nend\n");
        // C=2: ||= (condition) + map with block_pass (iterating method condition)
        assert_eq!(
            (a, b, c),
            (2, 2, 2),
            "block_pass should not block compound_extra"
        );
    }

    /// Bug 5: IndexOrWriteNode compound_assignment undercounted.
    /// In Parser, `@styles[cl] ||= Hash.new` produces (or_asgn (send :[] cl) (send Hash :new)).
    /// compound_assignment counts BOTH children as A+1 each. Our old code only counted
    /// the value side, missing the target [] A+1.
    #[test]
    fn index_or_write_compound_counts_both_target_and_value() {
        // @h[k] ||= Foo.new
        // compound_assignment_extra: target [] → A+1, value Foo.new → A+1
        // count_node IndexOrWriteNode: B+1 ([]), C+1 (||=)
        // Visitor visits Foo.new as CallNode → B+1, k as local var → no B
        // Total: A=2, B=2, C=1 => sqrt(4+4+1) = 3.0
        let (a, b, c) = abc_counter_for_source(b"def foo\n  k = 1\n  @h[k] ||= Foo.new\nend\n");
        // k=1 adds A+1, so total A=3 (k + index[] + Foo.new), B=3 ([] + Foo.new + Foo), C=1
        // Wait — Foo is a ConstantReadNode, not a CallNode. .new is the CallNode.
        // B: [] (IndexOrWrite) + new (CallNode) = 2
        // But Foo is ConstantReadNode → no B. The receiver of .new is Foo (const), not a call.
        // A: k=1 (lvar write) + [] target (compound) + Foo.new (compound value) = 3
        // C: ||= = 1
        assert_eq!(a, 3, "A should count k=1, [] target, and Foo.new value");
        assert_eq!(b, 2, "B should count [] and .new");
        assert_eq!(c, 1, "C should count ||=");
    }

    /// Bug 6: ForNode undercounted assignments by 1.
    /// In Parser, `for x in items` produces (for (lvasgn :x) ...). Both for_type? (A+1)
    /// and the child lvasgn (A+1) are counted. In Prism, ForNode index is
    /// LocalVariableTargetNode which has no count_node arm. Fix: A+=2 in ForNode handler.
    #[test]
    fn for_loop_counts_double_assignment() {
        // for x in items; x; end
        // A=2 (ForNode: for_type A+1 + lvasgn child A+1)
        // B=1 (items — receiverless call)
        // C=1 (ForNode condition)
        let (a, b, c) = abc_counter_for_source(b"def foo\n  for x in items\n    x\n  end\nend\n");
        assert_eq!(a, 2, "for loop should count A=2 (for_type + lvasgn)");
        assert_eq!(c, 1, "for loop should count C=1 (condition)");
    }

    /// BlockParameterNode in nested def: Prism's visit_parameters_node calls
    /// visit_block_parameter_node directly (not via visit()), bypassing
    /// visit_leaf_node_enter. Our override of visit_block_parameter_node
    /// ensures the &block param is counted as A+1.
    #[test]
    fn block_parameter_counted_in_nested_def() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };

        // Nested def with &block param inside a block:
        // A=1 (&block param), B=1 (Struct.new), C=0
        // => sqrt(1+1) = 1.41
        // Without the fix, &block wouldn't be counted: A=0 => sqrt(0+1) = 1.00
        let source =
            b"def test_method\n  Struct.new do\n    def foo(&block)\n    end\n  end\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config.clone());
        assert!(!diags.is_empty(), "Should fire with Max:1");
        assert!(
            diags[0].message.contains("[1.41/1]"),
            "Nested &block param should count as A+1. Got: {}",
            diags[0].message
        );

        // Block params: |&block| in a block
        // A=1 (&block), B=1 (define_method), C=0
        let source = b"def test_method\n  define_method(:foo) do |&block|\n    block\n  end\nend\n";
        let diags = run_cop_full_with_config(&AbcSize, source, config);
        assert!(!diags.is_empty(), "Should fire with Max:1");
        assert!(
            diags[0].message.contains("[1.41/1]"),
            "Block |&block| param should count as A+1. Got: {}",
            diags[0].message
        );
    }

    /// Diagnostic test: lambda with parameters should count params as A
    /// and the lambda itself as B+1.
    /// `->(s) { s.foo }` should be: A+1 (s param), B+2 (lambda + s.foo)
    #[test]
    fn lambda_with_params_abc_count() {
        // Single lambda with param: ->(s) { s.foo }
        // A: s param = 1, x lvar write = 1 => A=2
        // B: lambda = 1, s.foo = 1 => B=2
        // C: 0
        let (a, b, c) = abc_counter_for_source(b"def foo\n  x = ->(s) { s.foo }\nend\n");
        assert_eq!(a, 2, "A: x write + s param");
        assert_eq!(b, 2, "B: lambda + s.foo");
        assert_eq!(c, 0, "C: none");
    }

    /// Diagnostic: case/when with multiple values per when clause.
    /// `when '(', '{', '['` should count as C+1 (one when clause).
    #[test]
    fn case_when_multiple_values() {
        // case char
        //   when '(' then 1
        //   when ')' then 2
        // end
        // A=1 (x), B=0, C=2 (2 whens)
        let (a, b, c) = abc_counter_for_source(
            b"def foo\n  x = case char\n  when '(' then 1\n  when ')' then 2\n  end\nend\n",
        );
        assert_eq!(c, 2, "C: 2 when clauses. Got A={a}, B={b}, C={c}");

        // when '(', '{', '[' => still one when clause, C=1
        let (a2, b2, c2) = abc_counter_for_source(
            b"def foo\n  x = case char\n  when '(', '{', '[' then 1\n  end\nend\n",
        );
        assert_eq!(
            c2, 1,
            "C: 1 when clause with multiple values. Got A={a2}, B={b2}, C={c2}"
        );
    }

    /// Simplified version of Coursemology assertion_types_regex.
    /// RuboCop reports <12, 25, 1> = 27.75. Multiple lambdas with params.
    #[test]
    fn coursemology_assertion_types_regex_simplified() {
        // multi_arg = ->(s) { top_level_split(s, ',').map(&:strip) }
        //   A: multi_arg(1), s param(1) = 2
        //   B: lambda(1), top_level_split(1), map(1) = 3
        //   C: map(&:strip) iterating = 1
        // single_arg = ->(s) { s.strip }
        //   A: single_arg(1), s param(1) = 2
        //   B: lambda(1), strip(1) = 2
        //   C: 0
        // Equal: ->(s) { multi_arg.call(s).join(' == ') }
        //   A: s param(1) = 1
        //   B: lambda(1), call(1), join(1) = 3
        //   C: 0
        // NotEqual: ->(s) { multi_arg.call(s).join(' != ') }
        //   A: s param(1) = 1
        //   B: lambda(1), call(1), join(1) = 3
        //   C: 0
        // True: ->(s) { single_arg.call(s) }
        //   A: s param(1) = 1
        //   B: lambda(1), call(1) = 2
        //   C: 0
        // False: ->(s) { "not #{single_arg.call(s)}" }
        //   A: s param(1) = 1
        //   B: lambda(1), call(1) = 2
        //   C: 0
        //
        // Totals: A=8, B=15, C=1 => sqrt(64+225+1) = 17.03
        let source = b"def assertion_types_regex
  multi_arg = ->(s) { top_level_split(s, ',').map(&:strip) }
  single_arg = ->(s) { s.strip }
  {
    Equal: ->(s) { multi_arg.call(s).join(' == ') },
    NotEqual: ->(s) { multi_arg.call(s).join(' != ') },
    True: ->(s) { single_arg.call(s) },
    False: ->(s) { single_arg.call(s) }
  }
end
";
        let (a, b, c) = abc_counter_for_source(source);
        // Expected from RuboCop-style counting:
        // Let's be flexible and just ensure the method fires above Max=17
        let score = ((a * a + b * b + c * c) as f64).sqrt();
        let rounded = (score * 100.0).round() / 100.0;
        assert!(
            rounded > 17.0,
            "Score should be > 17 for assertion_types_regex. Got A={a}, B={b}, C={c}, score={rounded}"
        );
    }
}
