use crate::cop::node_type::CALL_NODE;
use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::method_complexity::{self, ComplexityScorer};

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
///
/// ## Corpus verification (2026-03-25)
///
/// verify_cop_locations.py: FP 0 fixed / 5 remain, FN 100 fixed / 0 remain.
/// All FN verified fixed. Remaining FP=5: auth0 (2), gisiahq (1),
/// noosfero (1), samvera (1) — all config resolution or vendored file issues.
/// No cop-level fix needed.
///
/// ## Inline enable directive fix (2026-03-29)
///
/// FP root cause: `end # rubocop:enable Cop` (inline/trailing enable) was
/// incorrectly closing block `# rubocop:disable Cop` directives. In RuboCop,
/// inline enables are no-ops — they do NOT close an open block disable. Only
/// standalone `# rubocop:enable Cop` (on its own line) closes a block disable.
/// This caused the samvera/hyrax FP: the file had a block disable for
/// CyclomaticComplexity with an inline enable on the `end` line, making the
/// disable extend to EOF in RuboCop but not in nitrocop.
/// Fix: skip inline enables in `src/parse/directives.rs`.
pub struct CyclomaticComplexity;

/// Cyclomatic scoring: every branch point counts as +1.
struct CyclomaticScorer;

impl ComplexityScorer for CyclomaticScorer {
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

// Config keys used via method_complexity::check_method_complexity:
// "Max", "AllowedMethods", "AllowedPatterns"

impl Cop for CyclomaticComplexity {
    fn name(&self) -> &'static str {
        "Metrics/CyclomaticComplexity"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, DEF_NODE]
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
        method_complexity::check_method_complexity(
            self,
            &CyclomaticScorer,
            "Cyclomatic complexity",
            source,
            node,
            config,
            diagnostics,
        );
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
