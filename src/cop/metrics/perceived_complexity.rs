use ruby_prism::Visit;

use crate::cop::node_type::{
    BLOCK_NODE, CALL_NODE, CASE_NODE, DEF_NODE, ELSE_NODE, IF_NODE, LOCAL_VARIABLE_READ_NODE,
    UNLESS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Metrics/PerceivedComplexity
///
/// ## Corpus investigation (2026-03-04)
///
/// Corpus oracle baseline reported FP=166 and FN=457. A local `check-cop --rerun`
/// after prior Metrics fixes still showed FN-only drift (missing offenses, no excess).
///
/// FN root causes fixed in this change:
/// - Prism represents inline rescue (`expr rescue fallback`) as
///   `RescueModifierNode`, but this cop only counted `RescueNode` chains.
/// - Rescue-chain tracking used a single boolean guard, which also suppressed
///   nested rescues inside rescue bodies (it should suppress only subsequent
///   clauses in the same chain).
///
/// Fix:
/// - Count `RescueModifierNode` as +1 decision point (same weight as rescue).
/// - Walk `RescueNode` chains manually so only `subsequent` clauses are
///   de-duplicated while nested rescues still contribute complexity.
///
/// Remaining gaps:
/// - Additional FN remain and require follow-up investigation on other
///   constructs beyond rescue modifiers.
///
/// ## FP fixes (2026-03-08)
///
/// Bug 1: KNOWN_ITERATING_METHODS had 6 extra methods not in RuboCop's
/// canonical list (each_line, each_byte, each_char, each_codepoint, rindex,
/// sort_by!). These caused false positives by over-counting block complexity.
/// Removed to match vendor/rubocop/lib/rubocop/cop/metrics/utils/iterating_block.rb.
///
/// Bug 2: CaseMatchNode (case/in pattern matching) was double-counted.
/// RuboCop's COUNTED_NODES includes :in_pattern but NOT :case_match, so each
/// InNode gets +1 individually without a CaseMatchNode formula on top.
/// Removed the CaseMatchNode arm from count_node() and interested_node_types.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=53, FN=397.
///
/// FN root cause identified: if/elsif scoring. In RuboCop (Parser gem),
/// `if...elsif...end` scores the outer `if` as +2 because Parser's `loc.else`
/// points to the `elsif` keyword, making `node.else?` return true. nitrocop
/// only scored +2 when the subsequent was an ElseNode, missing the elsif case.
///
/// An attempt was made to fix this by scoring +2 for any non-ternary,
/// non-elsif IfNode with any subsequent (commit 044592b8, reverted).
/// The fix correctly detects elsif via `if_keyword_loc().as_slice() == b"elsif"`.
/// However, when combined with the corpus environment (missing bundle symlinks
/// in worktree caused false 130K readings on BlockLength, masking the true
/// PerceivedComplexity validation). When re-validated with proper env, the fix
/// showed 0 excess over CI baseline and reduced FN from 397 to 241 (156 recovered).
///
/// The fix was reverted during batch integration due to initial false FAIL readings.
/// It should be re-applied and re-validated in a follow-up batch. The approach is
/// correct: score +2 for top-level `if` with any subsequent, +1 for elsif always.
/// Remaining 241 FN likely have other root causes (not if/elsif related).
pub struct PerceivedComplexity;

/// Known iterating method names that make blocks count toward complexity.
/// Sourced from RuboCop's Metrics::Utils::IteratingBlock::KNOWN_ITERATING_METHODS.
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

#[derive(Default)]
struct PerceivedCounter {
    complexity: usize,
    /// Tracks local variable names that have been seen with `&.` (safe navigation).
    /// RuboCop discounts repeated `&.` on the same variable — only the first counts.
    /// When the variable is reassigned, it is removed from the set (reset).
    seen_csend_vars: std::collections::HashSet<Vec<u8>>,
    /// Set when visiting an InNode's pattern to suppress counting guard
    /// IfNode/UnlessNode as separate decision points.
    in_pattern_guard: bool,
}

impl PerceivedCounter {
    fn count_node(&mut self, node: &ruby_prism::Node<'_>) {
        match node {
            // if with else (not elsif) counts as 2, otherwise 1
            // Ternary (x ? y : z) has no if_keyword_loc and counts as 1 (not 2).
            // Skip when in_pattern_guard — Prism wraps `in :x if guard` as
            // InNode(pattern=IfNode), and RuboCop's if_guard/unless_guard are not
            // in COUNTED_NODES, so the guard should not count separately.
            ruby_prism::Node::IfNode { .. } => {
                if !self.in_pattern_guard {
                    if let Some(if_node) = node.as_if_node() {
                        let is_ternary = if_node.if_keyword_loc().is_none();
                        if !is_ternary
                            && if_node
                                .subsequent()
                                .is_some_and(|s| s.as_else_node().is_some())
                        {
                            self.complexity += 2;
                        } else {
                            self.complexity += 1;
                        }
                    }
                }
            }
            // unless is a separate node type in Prism
            ruby_prism::Node::UnlessNode { .. } => {
                if !self.in_pattern_guard {
                    if let Some(unless_node) = node.as_unless_node() {
                        if unless_node.else_clause().is_some() {
                            self.complexity += 2;
                        } else {
                            self.complexity += 1;
                        }
                    }
                }
            }

            ruby_prism::Node::WhileNode { .. }
            | ruby_prism::Node::UntilNode { .. }
            | ruby_prism::Node::ForNode { .. }
            | ruby_prism::Node::AndNode { .. }
            | ruby_prism::Node::OrNode { .. }
            | ruby_prism::Node::RescueModifierNode { .. } => {
                self.complexity += 1;
            }
            // InNode is handled in visit_in_node to manage guard suppression.
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

            // or_asgn (||=) and and_asgn (&&=) count as conditions
            ruby_prism::Node::LocalVariableOrWriteNode { .. }
            | ruby_prism::Node::InstanceVariableOrWriteNode { .. }
            | ruby_prism::Node::ClassVariableOrWriteNode { .. }
            | ruby_prism::Node::GlobalVariableOrWriteNode { .. }
            | ruby_prism::Node::ConstantOrWriteNode { .. }
            | ruby_prism::Node::ConstantPathOrWriteNode { .. }
            | ruby_prism::Node::IndexOrWriteNode { .. }
            | ruby_prism::Node::CallOrWriteNode { .. }
            | ruby_prism::Node::LocalVariableAndWriteNode { .. }
            | ruby_prism::Node::InstanceVariableAndWriteNode { .. }
            | ruby_prism::Node::ClassVariableAndWriteNode { .. }
            | ruby_prism::Node::GlobalVariableAndWriteNode { .. }
            | ruby_prism::Node::ConstantAndWriteNode { .. }
            | ruby_prism::Node::ConstantPathAndWriteNode { .. }
            | ruby_prism::Node::IndexAndWriteNode { .. }
            | ruby_prism::Node::CallAndWriteNode { .. } => {
                self.complexity += 1;
            }

            // CallNode: count &. (safe navigation) and iterating blocks/block_pass
            ruby_prism::Node::CallNode { .. } => {
                if let Some(call) = node.as_call_node() {
                    // Safe navigation (&.) counts, but discount repeated &. on the same lvar
                    if call
                        .call_operator_loc()
                        .is_some_and(|loc| loc.as_slice() == b"&.")
                        && !self.discount_repeated_csend(&call)
                    {
                        self.complexity += 1;
                    }
                    // Iterating block or block_pass counts
                    if call.block().is_some_and(|b| {
                        b.as_block_node().is_some() || b.as_block_argument_node().is_some()
                    }) {
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

impl PerceivedCounter {
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

    /// Visit a rescue chain without adding extra complexity for subsequent clauses.
    /// Subsequent rescue clauses are siblings in Parser AST terms and should not add
    /// another decision point, but nested rescues in clause bodies should still count.
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

impl<'pr> Visit<'pr> for PerceivedCounter {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.count_node(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.count_node(&node);
    }

    // InNode: count +1 for the `in` clause, then visit children with guard
    // suppression. In Prism, `in :x if guard` wraps the pattern as IfNode
    // inside InNode, which would be double-counted without suppression.
    fn visit_in_node(&mut self, node: &ruby_prism::InNode<'pr>) {
        self.complexity += 1;
        // Visit the pattern with guard suppression active so that any
        // IfNode/UnlessNode guard is not counted as a separate decision point.
        self.in_pattern_guard = true;
        let pattern = node.pattern();
        self.visit(&pattern);
        self.in_pattern_guard = false;
        // Visit the body normally
        if let Some(stmts) = node.statements() {
            self.visit(&stmts.as_node());
        }
    }

    // When a local variable is reassigned, reset the csend tracking for it.
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.seen_csend_vars.remove(node.name().as_slice());
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    // RescueNode is visited via visit_rescue_node (not visit_branch_node_enter)
    // because Prism's visit_begin_node calls visitor.visit_rescue_node directly.
    // In Prism, rescue clauses are chained via `subsequent`, and each clause is a
    // separate RescueNode. RuboCop treats clauses in the same rescue chain as one
    // decision point, while nested rescues still count separately.
    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        self.complexity += 1;
        self.visit_rescue_chain(node);
    }
}

impl Cop for PerceivedComplexity {
    fn name(&self) -> &'static str {
        "Metrics/PerceivedComplexity"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BLOCK_NODE,
            CALL_NODE,
            CASE_NODE,
            DEF_NODE,
            ELSE_NODE,
            IF_NODE,
            LOCAL_VARIABLE_READ_NODE,
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
        if let Some(def_node) = node.as_def_node() {
            let body = match def_node.body() {
                Some(b) => b,
                None => return,
            };
            let method_name = def_node.name().as_slice();
            let start_offset = def_node.def_keyword_loc().start_offset();
            self.check_complexity(
                source,
                config,
                diagnostics,
                method_name,
                &body,
                start_offset,
            );
        } else if let Some(call_node) = node.as_call_node() {
            // Handle define_method(:name) do...end
            if call_node.name().as_slice() == b"define_method" && call_node.receiver().is_none() {
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
                        let start_offset = call_node.location().start_offset();
                        self.check_complexity(
                            source,
                            config,
                            diagnostics,
                            method_name.as_bytes(),
                            &body,
                            start_offset,
                        );
                    }
                }
            }
        }
    }
}

impl PerceivedComplexity {
    fn check_complexity(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        method_name_bytes: &[u8],
        body: &ruby_prism::Node<'_>,
        start_offset: usize,
    ) {
        let max = config.get_usize("Max", 8);

        // AllowedMethods / AllowedPatterns: skip methods matching these
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");
        let method_name_str = std::str::from_utf8(method_name_bytes).unwrap_or("");
        if let Some(allowed) = &allowed_methods {
            if allowed.iter().any(|m| m == method_name_str) {
                return;
            }
        }
        if let Some(patterns) = &allowed_patterns {
            if patterns
                .iter()
                .any(|p| regex::Regex::new(p).is_ok_and(|re| re.is_match(method_name_str)))
            {
                return;
            }
        }

        let mut counter = PerceivedCounter::default();
        counter.visit(body);

        let score = 1 + counter.complexity;
        if score > max {
            let method_name = std::str::from_utf8(method_name_bytes).unwrap_or("unknown");
            let (line, column) = source.offset_to_line_col(start_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Perceived complexity for {method_name} is too high. [{score}/{max}]"),
            ));
        }
    }
}

/// Extract the method name from a `define_method` call's first argument.
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

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PerceivedComplexity, "cops/metrics/perceived_complexity");

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // 1 (base) + 2 (if with else) = 3 > Max:1
        let source = b"def foo\n  if x\n    y\n  else\n    z\n  end\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire with Max:1 on method with if/else"
        );
        assert!(diags[0].message.contains("/1]"));
    }

    #[test]
    fn allowed_patterns_uses_regex() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([
                ("Max".into(), serde_yml::Value::Number(1.into())),
                (
                    "AllowedPatterns".into(),
                    serde_yml::Value::Sequence(vec![serde_yml::Value::String("^complex".into())]),
                ),
            ]),
            ..CopConfig::default()
        };
        // Method matching the regex pattern should be skipped
        let source = b"def complex_method\n  if x\n    y\n  else\n    z\n  end\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source, config);
        assert!(
            diags.is_empty(),
            "Should not fire on method matching AllowedPatterns regex"
        );
    }

    #[test]
    fn define_method_block_counted() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        let source = b"define_method(:foo) do\n  if x\n    y\n  else\n    z\n  end\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire on define_method block with complexity"
        );
        assert!(diags[0].message.contains("foo"));
    }

    #[test]
    fn block_pass_counted() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // base 1 + map(&:to_s) 1 = 2 > Max:1
        let source = b"def foo(items)\n  items.map(&:to_s)\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source, config);
        assert!(
            !diags.is_empty(),
            "Should count block_pass (&:method) in iterating methods"
        );
    }
}
