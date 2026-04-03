//! Shared infrastructure for method complexity cops (CyclomaticComplexity, PerceivedComplexity).
//!
//! Mirrors RuboCop's `MethodComplexity` mixin, `RepeatedCsendDiscount`, and
//! `Metrics::Utils::IteratingBlock`. Each cop provides a `ComplexityScorer` that
//! handles only the cop-specific node scoring; the shared `ComplexityVisitor`
//! handles everything else.

use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Known iterating method names that make blocks count toward complexity.
/// Must match RuboCop's `Metrics::Utils::IteratingBlock::KNOWN_ITERATING_METHODS`
/// (enumerable + enumerator + array + hash sets from iterating_block.rb).
pub const KNOWN_ITERATING_METHODS: &[&[u8]] = &[
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

// ── Scorer trait ────────────────────────────────────────────────────────

/// Cop-specific scoring for method complexity. Only the node types that differ
/// between CyclomaticComplexity and PerceivedComplexity need to be implemented.
pub trait ComplexityScorer {
    /// Score an IfNode. Only called when NOT inside a pattern guard.
    fn score_if(&self, node: &ruby_prism::IfNode<'_>) -> usize;
    /// Score an UnlessNode. Only called when NOT inside a pattern guard.
    fn score_unless(&self, node: &ruby_prism::UnlessNode<'_>) -> usize;
    /// Score a WhenNode. Cyclomatic: 1, Perceived: 0 (handled by case formula).
    fn score_when(&self) -> usize;
    /// Score a CaseNode. Cyclomatic: 0, Perceived: branch formula.
    fn score_case(&self, node: &ruby_prism::CaseNode<'_>) -> usize;
}

// ── Shared visitor ─────────────────────────────────────────────────────

/// AST visitor that computes method complexity using a pluggable scorer.
/// Handles all shared logic: csend discount, iterating block counting,
/// rescue chain dedup, pattern guard suppression, and lvar write reset.
pub struct ComplexityVisitor<'a, S: ComplexityScorer> {
    scorer: &'a S,
    pub complexity: usize,
    /// Tracks local variables seen with `&.` for repeated-csend discount.
    seen_csend_vars: HashSet<Vec<u8>>,
    /// Suppresses If/Unless counting inside InNode pattern guards.
    in_pattern_guard: bool,
    /// Tracks whether we are inside a rescue chain to avoid counting
    /// subsequent rescue clauses.
    in_rescue_chain: bool,
}

impl<'a, S: ComplexityScorer> ComplexityVisitor<'a, S> {
    pub fn new(scorer: &'a S) -> Self {
        Self {
            scorer,
            complexity: 0,
            seen_csend_vars: HashSet::new(),
            in_pattern_guard: false,
            in_rescue_chain: false,
        }
    }

    /// Process a node for complexity scoring. Handles both cop-specific scoring
    /// (via the scorer trait) and shared infrastructure (csend, iterating blocks,
    /// compound assignments).
    fn process_node(&mut self, node: &ruby_prism::Node<'_>) {
        match node {
            // ── Cop-specific scoring (delegated to scorer) ──────────
            ruby_prism::Node::IfNode { .. } => {
                if !self.in_pattern_guard {
                    if let Some(if_node) = node.as_if_node() {
                        self.complexity += self.scorer.score_if(&if_node);
                    }
                }
            }
            ruby_prism::Node::UnlessNode { .. } => {
                if !self.in_pattern_guard {
                    if let Some(unless_node) = node.as_unless_node() {
                        self.complexity += self.scorer.score_unless(&unless_node);
                    }
                }
            }
            ruby_prism::Node::WhenNode { .. } => {
                self.complexity += self.scorer.score_when();
            }
            ruby_prism::Node::CaseNode { .. } => {
                if let Some(case_node) = node.as_case_node() {
                    self.complexity += self.scorer.score_case(&case_node);
                }
            }

            // ── Shared scoring (identical for both cops) ────────────

            // While/Until: skip begin...end while/until (post-condition loops)
            ruby_prism::Node::WhileNode { .. } => {
                if let Some(while_node) = node.as_while_node() {
                    if !while_node.is_begin_modifier() {
                        self.complexity += 1;
                    }
                }
            }
            ruby_prism::Node::UntilNode { .. } => {
                if let Some(until_node) = node.as_until_node() {
                    if !until_node.is_begin_modifier() {
                        self.complexity += 1;
                    }
                }
            }

            ruby_prism::Node::ForNode { .. }
            | ruby_prism::Node::AndNode { .. }
            | ruby_prism::Node::OrNode { .. }
            | ruby_prism::Node::RescueModifierNode { .. } => {
                self.complexity += 1;
            }

            // Or/And compound assignments
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
            | ruby_prism::Node::ConstantPathAndWriteNode { .. }
            | ruby_prism::Node::IndexOrWriteNode { .. }
            | ruby_prism::Node::IndexAndWriteNode { .. }
            | ruby_prism::Node::CallOrWriteNode { .. }
            | ruby_prism::Node::CallAndWriteNode { .. } => {
                self.complexity += 1;
            }

            // CallNode: safe navigation discount + iterating block counting
            ruby_prism::Node::CallNode { .. } => {
                if let Some(call) = node.as_call_node() {
                    self.process_call_node(&call);
                }
            }

            // Reset csend tracking when a local variable is reassigned
            ruby_prism::Node::LocalVariableWriteNode { .. } => {
                if let Some(write) = node.as_local_variable_write_node() {
                    self.seen_csend_vars.remove(write.name().as_slice());
                }
            }

            _ => {}
        }
    }

    /// Handle CallNode: safe navigation (&.) discount and iterating block counting.
    fn process_call_node(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Safe navigation (&.) counts, with repeated csend discount:
        // Only count the first &. on each local variable receiver.
        if call
            .call_operator_loc()
            .is_some_and(|loc| loc.as_slice() == b"&.")
        {
            let should_count = if let Some(receiver) = call.receiver() {
                if let Some(lvar) = receiver.as_local_variable_read_node() {
                    let var_name = lvar.name().as_slice().to_vec();
                    self.seen_csend_vars.insert(var_name)
                } else {
                    true
                }
            } else {
                true
            };
            if should_count {
                self.complexity += 1;
            }
        }

        // Iterating block or block_pass counts.
        // RuboCop's Parser gem produces :numblock/:itblock for numbered/it params,
        // neither of which is in COUNTED_NODES. In Prism all blocks are BlockNode,
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
                block.as_block_argument_node().is_some()
            };
            if should_count {
                let method_name = call.name().as_slice();
                if KNOWN_ITERATING_METHODS.contains(&method_name) {
                    self.complexity += 1;
                }
            }
        }
    }

    /// Walk a rescue chain without counting subsequent clauses.
    fn visit_rescue_chain<'pr>(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        for exception in &node.exceptions() {
            self.visit(&exception);
        }
        if let Some(reference) = node.reference() {
            self.visit(&reference);
        }
        if let Some(statements) = node.statements() {
            self.visit_statements_node(&statements);
        }
        if let Some(subsequent) = node.subsequent() {
            self.visit_rescue_chain(&subsequent);
        }
    }
}

impl<'pr, S: ComplexityScorer> Visit<'pr> for ComplexityVisitor<'_, S> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.process_node(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.process_node(&node);
    }

    // RescueNode: count +1 for the first in a chain, walk children manually
    // to avoid counting subsequent clauses as separate decision points.
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        if !self.in_rescue_chain {
            self.complexity += 1;
            self.in_rescue_chain = true;
            self.visit_rescue_chain(node);
            self.in_rescue_chain = false;
        } else {
            self.visit_rescue_chain(node);
        }
    }

    // Reset in_rescue_chain for nested begin...rescue...end blocks
    // so they count as separate decision points.
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let saved = self.in_rescue_chain;
        self.in_rescue_chain = false;
        ruby_prism::visit_begin_node(self, node);
        self.in_rescue_chain = saved;
    }

    // InNode: count +1 for the `in` clause, visit children with guard
    // suppression so pattern-guard If/UnlessNodes aren't double-counted.
    fn visit_in_node(&mut self, node: &ruby_prism::InNode<'pr>) {
        self.complexity += 1;
        self.in_pattern_guard = true;
        let pattern = node.pattern();
        self.visit(&pattern);
        self.in_pattern_guard = false;
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
    }
}

// ── Shared check_node logic ────────────────────────────────────────────

/// Extract the method name from a `define_method` call's first argument.
pub fn extract_define_method_name(call: &ruby_prism::CallNode<'_>) -> Option<String> {
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

/// Shared `check_node` implementation for both complexity cops.
/// Extracts method name/body from DefNode or define_method, checks
/// AllowedMethods/AllowedPatterns, runs the visitor, and reports if over max.
pub fn check_method_complexity<S: ComplexityScorer>(
    cop: &dyn Cop,
    scorer: &S,
    complexity_label: &str,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Extract method name, body, and report location from DefNode or
    // define_method CallNode with block.
    let (method_name_str, body, report_offset) = if let Some(def_node) = node.as_def_node() {
        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };
        let name = std::str::from_utf8(def_node.name().as_slice())
            .unwrap_or("")
            .to_string();
        (name, body, def_node.def_keyword_loc().start_offset())
    } else if let Some(call_node) = node.as_call_node() {
        if call_node.name().as_slice() != b"define_method" || call_node.receiver().is_some() {
            return;
        }
        if let Some(block) = call_node.block() {
            if let Some(block_node) = block.as_block_node() {
                let method_name = match extract_define_method_name(&call_node) {
                    Some(name) => name,
                    None => return,
                };
                let body = match block_node.body() {
                    Some(b) => b,
                    None => return,
                };
                (method_name, body, call_node.location().start_offset())
            } else {
                return;
            }
        } else {
            return;
        }
    } else {
        return;
    };

    let max = config.get_usize(
        "Max",
        if complexity_label.starts_with('C') {
            7
        } else {
            8
        },
    );

    // AllowedMethods / AllowedPatterns
    let allowed_methods = config.get_string_array("AllowedMethods");
    let allowed_patterns = config.get_string_array("AllowedPatterns");
    if let Some(allowed) = &allowed_methods {
        if allowed.iter().any(|m| m == &method_name_str) {
            return;
        }
    }
    if let Some(patterns) = &allowed_patterns {
        if patterns
            .iter()
            .any(|p| regex::Regex::new(p).is_ok_and(|re| re.is_match(&method_name_str)))
        {
            return;
        }
    }

    let mut visitor = ComplexityVisitor::new(scorer);
    visitor.visit(&body);

    let score = 1 + visitor.complexity;
    if score > max {
        let (line, column) = source.offset_to_line_col(report_offset);
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("{complexity_label} for {method_name_str} is too high. [{score}/{max}]"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verify the constant matches what we expect
    #[test]
    fn known_iterating_methods_has_expected_entries() {
        assert!(KNOWN_ITERATING_METHODS.contains(&&b"map"[..]));
        assert!(KNOWN_ITERATING_METHODS.contains(&&b"each"[..]));
        assert!(KNOWN_ITERATING_METHODS.contains(&&b"select"[..]));
        assert!(KNOWN_ITERATING_METHODS.contains(&&b"transform_keys"[..]));
        assert!(KNOWN_ITERATING_METHODS.contains(&&b"with_index"[..]));
        assert!(KNOWN_ITERATING_METHODS.contains(&&b"fetch"[..]));
        assert!(!KNOWN_ITERATING_METHODS.contains(&&b"puts"[..]));
        assert!(!KNOWN_ITERATING_METHODS.contains(&&b"new"[..]));
    }

    // Test with a simple scorer that counts +1 for everything
    struct SimpleScorer;
    impl ComplexityScorer for SimpleScorer {
        fn score_if(&self, _node: &ruby_prism::IfNode<'_>) -> usize {
            1
        }
        fn score_unless(&self, _node: &ruby_prism::UnlessNode<'_>) -> usize {
            1
        }
        fn score_when(&self) -> usize {
            1
        }
        fn score_case(&self, _node: &ruby_prism::CaseNode<'_>) -> usize {
            0
        }
    }

    fn compute_complexity(source: &str) -> usize {
        let bytes = source.as_bytes().to_vec();
        let leaked = Box::leak(bytes.into_boxed_slice());
        let result = ruby_prism::parse(leaked);
        let scorer = SimpleScorer;
        let mut visitor = ComplexityVisitor::new(&scorer);
        visitor.visit(&result.node());
        visitor.complexity
    }

    #[test]
    fn simple_if_scores_one() {
        assert_eq!(compute_complexity("if x; y; end"), 1);
    }

    #[test]
    fn and_or_score_one_each() {
        assert_eq!(compute_complexity("x && y || z"), 2);
    }

    #[test]
    fn while_scores_one() {
        assert_eq!(compute_complexity("while x; y; end"), 1);
    }

    #[test]
    fn begin_end_while_not_counted() {
        assert_eq!(compute_complexity("begin; x; end while cond"), 0);
    }

    #[test]
    fn rescue_chain_scores_one() {
        // rescue with multiple clauses = 1 decision point
        assert_eq!(
            compute_complexity("begin; x; rescue A; y; rescue B; z; end"),
            1
        );
    }

    #[test]
    fn nested_rescue_scores_separately() {
        // Outer rescue + nested rescue in body = 2
        assert_eq!(
            compute_complexity("begin; x; rescue; begin; y; rescue; z; end; end"),
            2
        );
    }

    #[test]
    fn safe_navigation_counted_once_per_var() {
        // foo&.bar + foo&.baz = 1 (repeated csend on same var)
        assert_eq!(compute_complexity("foo = 1; foo&.bar; foo&.baz"), 1);
    }

    #[test]
    fn safe_navigation_reset_on_reassign() {
        // foo&.bar, foo = 2, foo&.baz = 2 (reset after reassign)
        assert_eq!(
            compute_complexity("foo = 1; foo&.bar; foo = 2; foo&.baz"),
            2
        );
    }

    #[test]
    fn iterating_block_counted() {
        // items.map { |x| x } = 1 (iterating block)
        assert_eq!(compute_complexity("items.map { |x| x }"), 1);
    }

    #[test]
    fn numbered_param_block_not_counted() {
        // items.map { _1 } = 0 (numblock, not counted)
        assert_eq!(compute_complexity("items.map { _1 }"), 0);
    }

    #[test]
    fn block_pass_counted() {
        // items.map(&:to_s) = 1
        assert_eq!(compute_complexity("items.map(&:to_s)"), 1);
    }

    #[test]
    fn non_iterating_block_not_counted() {
        // foo { |x| x } where foo is not in KNOWN_ITERATING_METHODS = 0
        assert_eq!(compute_complexity("foo { |x| x }"), 0);
    }

    #[test]
    fn or_assign_counted() {
        assert_eq!(compute_complexity("x = nil; x ||= 1"), 1);
    }

    #[test]
    fn pattern_guard_not_double_counted() {
        // case/in with guard: InNode counts +1, guard IfNode suppressed
        assert_eq!(compute_complexity("case x; in :a if y; z; end"), 1);
    }

    #[test]
    fn extract_define_method_name_symbol() {
        let src = b"define_method(:foo) {}";
        let leaked = Box::leak(src.to_vec().into_boxed_slice());
        let result = ruby_prism::parse(leaked);
        let program = result.node().as_program_node().unwrap();
        let first = program.statements().body().iter().next().unwrap();
        let call = first.as_call_node().unwrap();
        assert_eq!(extract_define_method_name(&call), Some("foo".to_string()));
    }

    #[test]
    fn extract_define_method_name_string() {
        let src = b"define_method('bar') {}";
        let leaked = Box::leak(src.to_vec().into_boxed_slice());
        let result = ruby_prism::parse(leaked);
        let program = result.node().as_program_node().unwrap();
        let first = program.statements().body().iter().next().unwrap();
        let call = first.as_call_node().unwrap();
        assert_eq!(extract_define_method_name(&call), Some("bar".to_string()));
    }
}
