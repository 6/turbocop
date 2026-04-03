use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::shared::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::method_complexity::{self, ComplexityScorer};

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
/// Fix attempted: score +2 for any non-ternary, non-elsif IfNode with any
/// subsequent (commit 044592b8, reverted). The fix correctly detects elsif
/// via `if_keyword_loc().as_slice() == b"elsif"`.
///
/// Corpus validation result: recovers all 397 FN (0 missing) but introduces
/// 154 net FP (nitrocop 19,248 vs RuboCop 19,094), failing acceptance gate
/// with 76 excess over CI baseline (after 422 file-drop noise adjustment).
/// The fix is theoretically correct per RuboCop's scoring semantics, but
/// the higher if/elsif scores expose over-counting in other areas that were
/// previously compensated by the under-counting of if/elsif chains.
///
/// A correct fix needs to simultaneously address the other over-counting
/// sources (investigate the 154 FP locations) so the net effect is 0 excess.
/// The if/elsif approach itself is correct; the problem is combinatorial
/// interaction with other scoring differences.
///
/// ## Corpus investigation (2026-03-09)
///
/// Re-ran the cop under the repository's Ruby 3.4 toolchain:
/// `mise exec ruby@3.4 -- python3 scripts/check-cop.py
/// Metrics/PerceivedComplexity --verbose --rerun`.
///
/// Result:
/// - Expected: 19,094
/// - Actual:   18,853
/// - Excess:   0 over CI baseline after file-drop adjustment
/// - Missing:  241
///
/// No code change was taken in this run. The cop is still a real FN-only
/// candidate, but the excess side is now clean under a proper rerun
/// environment, so future work should focus on recovering the remaining 241
/// missing offenses without reopening FP regressions.
///
/// ## Fix (2026-03-09) — if/elsif scoring + numblock over-counting
///
/// Root cause #1 (FN): if/elsif scoring. RuboCop scores +2 for any non-ternary,
/// non-elsif IfNode with ANY subsequent (else or elsif). nitrocop only scored +2
/// when subsequent was ElseNode, missing elsif. Fix: check `subsequent().is_some()`
/// and `!is_elsif` instead of `subsequent().as_else_node().is_some()`.
///
/// Root cause #2 (latent FP): numbered-param blocks (_1) and `it` blocks were
/// counted as iterating blocks. RuboCop uses :numblock/:itblock types not in
/// COUNTED_NODES. In Prism all blocks are BlockNode. Fix: check parameters()
/// for NumberedParametersNode/ItParametersNode (same pattern as CyclomaticComplexity).
///
/// Both fixes applied simultaneously to avoid net FP regression that occurred
/// when only fix #1 was applied in isolation (commit 044592b8, previously reverted).
///
/// ## FP fix (2026-03-10) — begin...end while/until overcounting
///
/// Root cause: In the Parser gem, `begin...end while cond` produces `:while_post`
/// and `begin...end until cond` produces `:until_post`, which are NOT included in
/// COUNTED_NODES. In Prism, both forms are WhileNode/UntilNode with the
/// `begin_modifier` flag set. nitrocop was counting these as +1, but they should
/// be skipped entirely. Fix: check `is_begin_modifier()` and skip counting.
///
/// Affected repos: SquareSquash/web (bin/setup:149), discourse (config.rb:105),
/// huginn (switch_to_json_serialization.rb:45), optcarrot (apu.rb:559).
/// All 4 FPs had score 9 vs threshold 8, overcounted by exactly 1.
///
/// ## Extended corpus investigation (2026-03-23)
///
/// Extended corpus (5592 repos) reported FP=31, FN=0. Standard corpus is 0/0.
///
/// FP=31 root cause: same cross-cutting file-level issue as CyclomaticComplexity.
/// 27/31 FP from the same 2 repos (Tubalr and stackneveroverflow) with vendored
/// gems that RuboCop cannot parse but Prism handles. Remaining 4 FP from auth0 (2),
/// gisiahq (1), pitluga (1) — likely config resolution differences.
/// No cop-level fix needed; requires infrastructure fix.
///
/// ## Corpus verification (2026-03-25)
///
/// verify_cop_locations.py: FP 0 fixed / 3 remain, FN 100 fixed / 0 remain.
/// All FN verified fixed. Remaining FP=3: auth0 (2), gisiahq (1) — config
/// resolution differences. No cop-level fix needed.
pub struct PerceivedComplexity;

/// Perceived scoring: if+else=2, case/when uses branch formula, etc.
struct PerceivedScorer;

impl ComplexityScorer for PerceivedScorer {
    fn score_if(&self, node: &ruby_prism::IfNode<'_>) -> usize {
        let is_ternary = node.if_keyword_loc().is_none();
        let is_elsif = node
            .if_keyword_loc()
            .is_some_and(|loc| loc.as_slice() == b"elsif");
        if !is_ternary && !is_elsif && node.subsequent().is_some() {
            2
        } else {
            1
        }
    }

    fn score_unless(&self, node: &ruby_prism::UnlessNode<'_>) -> usize {
        if node.else_clause().is_some() { 2 } else { 1 }
    }

    fn score_when(&self) -> usize {
        // WhenNode is NOT counted separately — CaseNode handles the scoring.
        0
    }

    fn score_case(&self, node: &ruby_prism::CaseNode<'_>) -> usize {
        let nb_whens = node.conditions().iter().count();
        let has_else = node.else_clause().is_some();
        let nb_branches = nb_whens + if has_else { 1 } else { 0 };

        if node.predicate().is_some() {
            // case expr; when ... -> 0.8 + 0.2 * branches
            ((nb_branches as f64 * 0.2) + 0.8).round() as usize
        } else {
            // case; when ... -> each when counts
            nb_branches
        }
    }
}

// Config keys used via method_complexity::check_method_complexity:
// "Max", "AllowedMethods", "AllowedPatterns"

impl Cop for PerceivedComplexity {
    fn name(&self) -> &'static str {
        "Metrics/PerceivedComplexity"
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
            &PerceivedScorer,
            "Perceived complexity",
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

    /// Numbered parameter blocks (_1) should NOT count as iterating blocks.
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
        let diags = run_cop_full_with_config(&PerceivedComplexity, source_regular, config.clone());
        assert!(
            diags[0].message.contains("[2/0]"),
            "Regular block should count: got {}",
            diags[0].message
        );

        // Numbered param block: map { _1 } should NOT count
        let source_numblock = b"def foo\n  items.map { _1 }\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source_numblock, config.clone());
        assert!(
            diags[0].message.contains("[1/0]"),
            "Numbered param block should NOT count: got {}",
            diags[0].message
        );

        // `it` block: map { it } should NOT count
        let source_it = b"def foo\n  items.map { it }\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source_it, config.clone());
        assert!(
            diags[0].message.contains("[1/0]"),
            "`it` block should NOT count: got {}",
            diags[0].message
        );

        // No-param block: map { 42 } should still count
        let source_noparam = b"def foo\n  items.map { 42 }\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source_noparam, config.clone());
        assert!(
            diags[0].message.contains("[2/0]"),
            "No-param block should count: got {}",
            diags[0].message
        );
    }

    /// if with elsif should score +2 for the outer if and +1 for each elsif.
    #[test]
    fn if_elsif_scores_two_for_outer_if() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(1.into()))]),
            ..CopConfig::default()
        };

        // if with elsif: outer if scores 2 + elsif scores 1 = base 1 + 3 = 4
        let source = b"def foo\n  if x\n    a\n  elsif y\n    b\n  end\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source, config.clone());
        assert!(!diags.is_empty(), "if/elsif should fire");
        assert!(
            diags[0].message.contains("[4/1]"),
            "Expected [4/1] got: {}",
            diags[0].message
        );

        // if with else (not elsif): scores 2 = base 1 + 2 = 3
        let source_else = b"def foo\n  if x\n    a\n  else\n    b\n  end\nend\n";
        let diags = run_cop_full_with_config(&PerceivedComplexity, source_else, config.clone());
        assert!(
            diags[0].message.contains("[3/1]"),
            "Expected [3/1] got: {}",
            diags[0].message
        );

        // elsif itself should score 1, not 2 (even when it has an else)
        let source_elsif_else =
            b"def foo\n  if x\n    a\n  elsif y\n    b\n  else\n    c\n  end\nend\n";
        let diags =
            run_cop_full_with_config(&PerceivedComplexity, source_elsif_else, config.clone());
        assert!(
            diags[0].message.contains("[4/1]"),
            "Expected [4/1] for if/elsif/else, got: {}",
            diags[0].message
        );
    }
}
