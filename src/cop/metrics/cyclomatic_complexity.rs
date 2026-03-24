use ruby_prism::Visit;

use crate::cop::node_type::{
    CALL_NODE, DEF_NODE, LOCAL_VARIABLE_READ_NODE, LOCAL_VARIABLE_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Metrics/CyclomaticComplexity
///
/// Investigation: FP=150 FN=1,399 (as of 2026-03-03)
///
/// FN root causes (fixed):
/// - Missing `define_method` blocks: only DefNode was handled, but
///   `define_method(:name) do...end` is CallNode + BlockNode.
/// - Missing `block_pass` iterating methods: `items.map(&:foo)` uses
///   BlockArgumentNode, not BlockNode.
/// - Missing compound assignment nodes: `IndexOrWriteNode`,
///   `IndexAndWriteNode`, `CallOrWriteNode`, `CallAndWriteNode` were
///   not counted as conditions.
/// - Missing inline rescue handling: Prism models `expr rescue fallback`
///   as `RescueModifierNode`, which must count as a decision point.
///   Fixing this reduced local corpus rerun drift from missing=70 to missing=4
///   (expected=22,797, actual=22,793; potential FP remained 0) on 2026-03-04.
/// - KNOWN_ITERATING_METHODS list mismatch vs RuboCop: nitrocop was missing
///   32 methods (with_index, with_object, transform_keys, merge, fetch, etc.)
///   and had 10 extra methods (each_byte, each_line, sort_by!, uniq!, etc.).
///   Synced to match vendor/rubocop/lib/rubocop/cop/metrics/utils/iterating_block.rb
///   exactly (enumerable + enumerator + array + hash sets). FP=123->?, FN=191->?.
///
/// FP root causes (fixed):
/// - AllowedPatterns used substring match instead of regex.
/// - Pattern matching `if` guards: in `case/in` with `in :x if guard`,
///   Prism nests an IfNode inside InNode's pattern, causing double-counting.
/// - KNOWN_ITERATING_METHODS list had 10 extra methods not in RuboCop's list
///   (each_byte, each_char, each_codepoint, each_line, filter!, filter_map!,
///   flat_map!, rindex, sort_by!, uniq!), causing over-counting.
/// - Numbered parameter blocks (`_1`) and `it` blocks were counted as iterating
///   blocks, but RuboCop's Parser gem produces :numblock/:itblock (not :block)
///   for these, and neither is in COUNTED_NODES. In Prism all blocks are
///   BlockNode, so we check `parameters()` type to distinguish. This was the
///   dominant FP source (82 FP), especially in repos using modern Ruby idioms.
///
/// Reverted attempt:
/// - Counting nested rescues separately via manual rescue-chain traversal closed
///   remaining FN but introduced potential FP (+12 vs RuboCop expected offenses).
///   The manual traversal approach was reverted to preserve zero-excess behavior.
///
/// ## Corpus investigation (2026-03-10)
///
/// FP root cause: `begin...end while/until` (post-condition loops) were
/// counted as decision points. In Parser gem these are `:while_post`/`:until_post`
/// which are NOT in `COUNTED_NODES`. In Prism both forms are `WhileNode`/`UntilNode`
/// with the `begin_modifier` flag set. Fix: skip counting when `is_begin_modifier()`.
/// This resolved 10 FP across 8 repos (rank, advance_to, gets, cat, token, etc.).
///
/// FN root cause: Nested `begin...rescue...end` blocks inside rescue clause
/// bodies were not counted because the `in_rescue_chain` flag remained true.
/// Fix: override `visit_begin_node` to save/restore `in_rescue_chain`, so
/// nested rescue scopes start fresh. This resolved 14 FN across 12 repos.
///
/// ## Extended corpus investigation (2026-03-23)
///
/// Extended corpus (5592 repos) reported FP=33, FN=0. Standard corpus is 0/0.
///
/// FP=33 root cause: cross-cutting file-level issue, NOT a cop algorithm bug.
/// 27/33 FP come from 2 repos with vendored Ruby gems (cjstewart88/Tubalr at
/// heroku/ruby/1.9.1/gems/rdoc-3.8/ and liaoziyang/stackneveroverflow at
/// vendor/bundle/ruby/2.3.0/gems/rdoc-4.3.0/). RuboCop does not process these
/// files (likely parser incompatibility with old Ruby 1.9 syntax or encoding),
/// while nitrocop (Prism) parses them successfully. The same repos contribute
/// FPs across ALL Metrics cops and many other departments.
/// Remaining 6 FP from auth0 (2), gisiahq (1), noosfero (1), pitluga (1),
/// samvera (1) — likely config resolution differences (project .rubocop.yml
/// Max overrides or AllCops.Exclude patterns not loaded identically).
/// No cop-level fix needed; requires infrastructure fix for file exclusion
/// and config resolution parity.
pub struct CyclomaticComplexity;

#[derive(Default)]
struct CyclomaticCounter {
    complexity: usize,
    /// Tracks whether we are already inside a rescue chain to avoid
    /// counting subsequent rescue clauses (Prism chains them via `subsequent`).
    in_rescue_chain: bool,
    /// Tracks local variables that have been seen with `&.` (safe navigation).
    /// Only the first `&.` call on a variable counts; subsequent ones on the
    /// same variable are discounted (matching RuboCop's RepeatedCsendDiscount).
    seen_csend_vars: std::collections::HashSet<Vec<u8>>,
    /// Set when visiting an InNode's pattern to suppress counting guard
    /// IfNode/UnlessNode as separate decision points.
    in_pattern_guard: bool,
}

/// Known iterating method names that make blocks count toward complexity.
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

impl CyclomaticCounter {
    fn count_node(&mut self, node: &ruby_prism::Node<'_>) {
        match node {
            // Skip IfNode/UnlessNode when they are pattern guards inside InNode.
            // Prism wraps `in :x if guard` as InNode(pattern=IfNode(...)), so the
            // guard IfNode would be double-counted (InNode already counts +1).
            ruby_prism::Node::IfNode { .. } | ruby_prism::Node::UnlessNode { .. } => {
                if !self.in_pattern_guard {
                    self.complexity += 1;
                }
            }
            // In Parser gem, `begin...end while cond` produces :while_post
            // (and `begin...end until` produces :until_post), which are NOT in
            // COUNTED_NODES. In Prism both forms are WhileNode/UntilNode with
            // the `begin_modifier` flag set. Skip counting when that flag is set.
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
            | ruby_prism::Node::WhenNode { .. }
            | ruby_prism::Node::AndNode { .. }
            | ruby_prism::Node::OrNode { .. }
            | ruby_prism::Node::RescueModifierNode { .. } => {
                self.complexity += 1;
            }
            // InNode is handled in visit_in_node to manage guard suppression.
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

            // Index and call compound assignments: h["key"] ||=, obj.attr &&=
            ruby_prism::Node::IndexOrWriteNode { .. }
            | ruby_prism::Node::IndexAndWriteNode { .. }
            | ruby_prism::Node::CallOrWriteNode { .. }
            | ruby_prism::Node::CallAndWriteNode { .. } => {
                self.complexity += 1;
            }

            // CallNode: count &. (safe navigation) and iterating blocks/block_pass
            ruby_prism::Node::CallNode { .. } => {
                if let Some(call) = node.as_call_node() {
                    // Safe navigation (&.) counts, with repeated csend discount:
                    // Only count the first &. on each local variable receiver.
                    if call
                        .call_operator_loc()
                        .is_some_and(|loc| loc.as_slice() == b"&.")
                    {
                        let should_count = if let Some(receiver) = call.receiver() {
                            if let Some(lvar) = receiver.as_local_variable_read_node() {
                                // First time seeing this variable with &.?
                                let var_name = lvar.name().as_slice().to_vec();
                                self.seen_csend_vars.insert(var_name)
                            } else {
                                true // Non-local-variable receivers always count
                            }
                        } else {
                            true
                        };
                        if should_count {
                            self.complexity += 1;
                        }
                    }
                    // Iterating block or block_pass counts.
                    // Note: RuboCop's Parser gem produces :numblock for numbered
                    // parameter blocks (_1, _2) and :itblock for `it` blocks,
                    // neither of which is in COUNTED_NODES. Only regular :block
                    // and :block_pass count. In Prism all blocks are BlockNode,
                    // so we check parameters to distinguish.
                    if let Some(block) = call.block() {
                        let should_count = if let Some(block_node) = block.as_block_node() {
                            // Skip blocks with numbered parameters (_1) or `it` params
                            match block_node.parameters() {
                                Some(params) => {
                                    params.as_numbered_parameters_node().is_none()
                                        && params.as_it_parameters_node().is_none()
                                }
                                // No parameters — regular block, counts
                                None => true,
                            }
                        } else {
                            // BlockArgumentNode (&:method) — always counts
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
            }

            // Reset csend tracking when a local variable is reassigned
            ruby_prism::Node::LocalVariableWriteNode { .. } => {
                if let Some(write) = node.as_local_variable_write_node() {
                    let var_name = write.name().as_slice().to_vec();
                    self.seen_csend_vars.remove(&var_name);
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

    // Override visit_begin_node to reset in_rescue_chain before visiting the
    // begin's rescue clause. This ensures that nested begin...rescue...end
    // blocks inside a rescue body are counted as separate decision points.
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let saved = self.in_rescue_chain;
        self.in_rescue_chain = false;
        ruby_prism::visit_begin_node(self, node);
        self.in_rescue_chain = saved;
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

impl Cop for CyclomaticComplexity {
    fn name(&self) -> &'static str {
        "Metrics/CyclomaticComplexity"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            DEF_NODE,
            LOCAL_VARIABLE_READ_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
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
            // Handle define_method(:name) do...end
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

        let max = config.get_usize("Max", 7);

        // AllowedMethods / AllowedPatterns: skip methods matching these
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

        let mut counter = CyclomaticCounter::default();
        counter.visit(&body);

        let score = 1 + counter.complexity;
        if score > max {
            let (line, column) = source.offset_to_line_col(report_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Cyclomatic complexity for {method_name_str} is too high. [{score}/{max}]"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CyclomaticComplexity, "cops/metrics/cyclomatic_complexity");

    #[test]
    fn config_custom_max() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };
        // 1 (base) + 1 (if) = 2 > Max:1
        let source = b"def foo\n  if x\n    y\n  end\nend\n";
        let diags = run_cop_full_with_config(&CyclomaticComplexity, source, config);
        assert!(
            !diags.is_empty(),
            "Should fire with Max:1 on method with if branch"
        );
        assert!(diags[0].message.contains("[2/1]"));
    }

    /// `begin...end while` (post-condition loop) should NOT count as a decision
    /// point. In Parser gem these produce :while_post (not in COUNTED_NODES).
    #[test]
    fn begin_end_while_not_counted() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(0.into()))]),
            ..CopConfig::default()
        };

        // Regular while counts
        let source_while = b"def foo\n  while cond\n    x\n  end\nend\n";
        let diags = run_cop_full_with_config(&CyclomaticComplexity, source_while, config.clone());
        assert!(
            diags[0].message.contains("[2/0]"),
            "Regular while should count: got {}",
            diags[0].message
        );

        // begin...end while does NOT count
        let source_post = b"def foo\n  begin\n    x\n  end while cond\nend\n";
        let diags = run_cop_full_with_config(&CyclomaticComplexity, source_post, config.clone());
        assert!(
            diags[0].message.contains("[1/0]"),
            "Post-condition while should NOT count: got {}",
            diags[0].message
        );

        // begin...end until does NOT count
        let source_until = b"def foo\n  begin\n    x\n  end until cond\nend\n";
        let diags = run_cop_full_with_config(&CyclomaticComplexity, source_until, config.clone());
        assert!(
            diags[0].message.contains("[1/0]"),
            "Post-condition until should NOT count: got {}",
            diags[0].message
        );

        // Regular until counts
        let source_until_pre = b"def foo\n  until cond\n    x\n  end\nend\n";
        let diags =
            run_cop_full_with_config(&CyclomaticComplexity, source_until_pre, config.clone());
        assert!(
            diags[0].message.contains("[2/0]"),
            "Regular until should count: got {}",
            diags[0].message
        );
    }

    /// Nested rescue inside a rescue body should count as a separate decision
    /// point. The outer rescue chain flag must not suppress inner rescues.
    #[test]
    fn nested_rescue_in_rescue_body_counted() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(0.into()))]),
            ..CopConfig::default()
        };

        // Outer rescue + nested rescue in body = 2 decision points
        let source = b"def foo\n  begin\n    x\n  rescue => e\n    begin\n      y\n    rescue\n      z\n    end\n  end\nend\n";
        let diags = run_cop_full_with_config(&CyclomaticComplexity, source, config.clone());
        assert!(
            diags[0].message.contains("[3/0]"),
            "Outer + nested rescue should be 3: got {}",
            diags[0].message
        );
    }

    /// Numbered parameter blocks (_1) should NOT count as iterating blocks.
    /// RuboCop's Parser gem produces :numblock (not :block) for these, and
    /// :numblock is not in COUNTED_NODES.
    #[test]
    fn numblock_not_counted_as_iterating() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(0.into()))]),
            ..CopConfig::default()
        };

        // Regular block: map { |x| x } should count +1
        let source_regular = b"def foo\n  items.map { |x| x }\nend\n";
        let diags = run_cop_full_with_config(&CyclomaticComplexity, source_regular, config.clone());
        assert!(
            diags[0].message.contains("[2/0]"),
            "Regular block should count: got {}",
            diags[0].message
        );

        // Numbered param block: map { _1 } should NOT count
        let source_numblock = b"def foo\n  items.map { _1 }\nend\n";
        let diags =
            run_cop_full_with_config(&CyclomaticComplexity, source_numblock, config.clone());
        assert!(
            diags[0].message.contains("[1/0]"),
            "Numbered param block should NOT count: got {}",
            diags[0].message
        );

        // `it` block: map { it } should NOT count
        let source_it = b"def foo\n  items.map { it }\nend\n";
        let diags = run_cop_full_with_config(&CyclomaticComplexity, source_it, config.clone());
        assert!(
            diags[0].message.contains("[1/0]"),
            "`it` block should NOT count: got {}",
            diags[0].message
        );

        // No-param block: map { 42 } should still count (it's a regular :block in Parser)
        let source_noparam = b"def foo\n  items.map { 42 }\nend\n";
        let diags = run_cop_full_with_config(&CyclomaticComplexity, source_noparam, config.clone());
        assert!(
            diags[0].message.contains("[2/0]"),
            "No-param block should count: got {}",
            diags[0].message
        );

        // block_pass: map(&:to_s) should still count regardless
        let source_blockpass = b"def foo\n  items.map(&:to_s)\nend\n";
        let diags =
            run_cop_full_with_config(&CyclomaticComplexity, source_blockpass, config.clone());
        assert!(
            diags[0].message.contains("[2/0]"),
            "Block-pass should count: got {}",
            diags[0].message
        );
    }
}
