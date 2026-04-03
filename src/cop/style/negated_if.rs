use crate::cop::shared::node_type::{CALL_NODE, IF_NODE};
use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/NegatedIf flags `if !condition` and `if not condition` (both prefix
/// and modifier forms) and suggests using `unless` instead.
///
/// Key behaviors matching RuboCop's NegativeConditional mixin:
/// - Unwraps parentheses around the condition before checking for negation
///   (e.g. `if (!foo)` and `if (not bar)` are flagged)
/// - Skips double negation `!!` (not a true negation, it's a boolean cast)
/// - Skips `if/elsif/else` chains (only bare `if` without else)
/// - Reports at the start of the full if-node (for modifier form, this is the
///   start of the body expression, not the `if` keyword)
///
/// Root causes of prior FPs/FNs:
/// - FPs: `!!condition` was not excluded (double negation)
/// - FNs: parenthesized conditions `if (!cond)` were missed because
///   `predicate.as_call_node()` returned None for the ParenthesesNode wrapper
/// - FPs/FNs: modifier-form location was reported at `if` keyword instead of
///   at the start of the full expression, causing line mismatches vs RuboCop
/// - FPs (4 in danbooru): pattern match guards `in :pattern if !condition` were
///   flagged, but `unless` is invalid syntax in guard clauses. Fixed by detecting
///   modifier IfNodes preceded by `in ` in the source bytes.
/// - FPs (1): safe-navigation chains ending in `&.!` (e.g. `obj&.empty?&.!`)
///   were flagged. Rewriting to `unless` with safe-nav is problematic. Fixed by
///   checking `call_operator_loc` on the `!` CallNode.
pub struct NegatedIf;

impl Cop for NegatedIf {
    fn name(&self) -> &'static str {
        "Style/NegatedIf"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, IF_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "both");
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return,
        };

        // Must have an `if` keyword (not ternary)
        let if_kw_loc = match if_node.if_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Must actually be `if`, not `unless`
        if if_kw_loc.as_slice() != b"if" {
            return;
        }

        // Must not have an else/elsif clause
        if if_node.subsequent().is_some() {
            return;
        }

        // Detect modifier (postfix) form: `do_something if condition`
        let is_modifier = if_node.end_keyword_loc().is_none();

        // Skip pattern match guards: `in :pattern if !condition`
        // Prism wraps pattern guards as modifier IfNodes inside InNode conditions.
        // These cannot use `unless` — the syntax `in :x unless cond` is invalid Ruby.
        // Detect by checking if the source bytes before the IfNode start with `in `.
        if is_modifier {
            let start = node.location().start_offset();
            let bytes = source.as_bytes();
            // Walk backwards from the IfNode start past whitespace to find `in`
            let mut pos = start;
            while pos > 0 && (bytes[pos - 1] == b' ' || bytes[pos - 1] == b'\t') {
                pos -= 1;
            }
            if pos >= 2 && bytes[pos - 2..pos] == *b"in" {
                // Verify it's a word boundary (start of line or preceded by whitespace/newline)
                if pos == 2
                    || bytes[pos - 3] == b'\n'
                    || bytes[pos - 3] == b' '
                    || bytes[pos - 3] == b'\t'
                {
                    return;
                }
            }
        }

        // EnforcedStyle filtering
        match enforced_style {
            "prefix" if is_modifier => return,
            "postfix" if !is_modifier => return,
            _ => {} // "both" checks all forms
        }

        // Unwrap parentheses from the predicate, then check for single negation
        let predicate = if_node.predicate();
        let unwrapped = util::unwrap_parentheses(predicate);

        if util::is_single_negation(&unwrapped) {
            // Report at the start of the full if-node expression.
            // For modifier form `body if !cond`, this is the start of `body`.
            // For prefix form `if !cond`, this is the `if` keyword.
            let (line, column) = source.offset_to_line_col(node.location().start_offset());
            let mut diag = self.diagnostic(
                source,
                line,
                column,
                "Favor `unless` over `if` for negative conditions.".to_string(),
            );

            // Autocorrect: replace `if` with `unless` and remove `!`/`not` from condition
            if let Some(ref mut corr) = corrections {
                // 1. Replace `if` keyword with `unless`
                corr.push(crate::correction::Correction {
                    start: if_kw_loc.start_offset(),
                    end: if_kw_loc.end_offset(),
                    replacement: "unless".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });

                // 2. Replace the negated condition with its inner expression
                // The predicate may be wrapped in parens, so we work with the original predicate
                let predicate = if_node.predicate();
                let pred_start = predicate.location().start_offset();
                let pred_end = predicate.location().end_offset();

                // Get the inner expression (without negation and optional parens)
                let inner_expr = util::get_negation_inner(&unwrapped);
                if let Some(inner) = inner_expr {
                    let inner_src = std::str::from_utf8(inner.location().as_slice())
                        .unwrap_or("")
                        .to_string();
                    // Add a space prefix if there's no space between keyword and predicate
                    let needs_space = pred_start == if_kw_loc.end_offset();
                    let replacement = if needs_space {
                        format!(" {inner_src}")
                    } else {
                        inner_src
                    };
                    corr.push(crate::correction::Correction {
                        start: pred_start,
                        end: pred_end,
                        replacement,
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                }

                diag.corrected = true;
            }

            diagnostics.push(diag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;

    crate::cop_fixture_tests!(NegatedIf, "cops/style/negated_if");
    crate::cop_autocorrect_fixture_tests!(NegatedIf, "cops/style/negated_if");

    #[test]
    fn parenthesized_negation() {
        use crate::testutil::run_cop_full;
        let source = b"if (!foo)\n  bar\nend\n";
        let diags = run_cop_full(&NegatedIf, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag parenthesized negation: {:?}",
            diags
        );
    }

    #[test]
    fn parenthesized_negation_no_space() {
        use crate::testutil::run_cop_full;
        let source = b"if(!foo)\n  bar\nend\n";
        let diags = run_cop_full(&NegatedIf, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag parenthesized negation without space: {:?}",
            diags
        );
    }

    #[test]
    fn double_negation_not_flagged() {
        use crate::testutil::run_cop_full;
        let source = b"if !!condition\n  do_something\nend\n";
        let diags = run_cop_full(&NegatedIf, source);
        assert_eq!(
            diags.len(),
            0,
            "Should NOT flag double negation: {:?}",
            diags
        );
    }

    #[test]
    fn not_keyword() {
        use crate::testutil::run_cop_full;
        let source = b"if not condition\n  do_something\nend\n";
        let diags = run_cop_full(&NegatedIf, source);
        assert_eq!(diags.len(), 1, "Should flag 'not' keyword: {:?}", diags);
    }

    #[test]
    fn modifier_double_negation_not_flagged() {
        use crate::testutil::run_cop_full;
        let source = b"return if !!ENV[\"testing\"]\n";
        let diags = run_cop_full(&NegatedIf, source);
        assert_eq!(
            diags.len(),
            0,
            "Should NOT flag modifier double negation: {:?}",
            diags
        );
    }

    #[test]
    fn modifier_parenthesized_negation() {
        use crate::testutil::run_cop_full;
        let source = b"something if (!x.even?)\n";
        let diags = run_cop_full(&NegatedIf, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag modifier parenthesized negation: {:?}",
            diags
        );
    }

    #[test]
    fn multiline_modifier_position() {
        use crate::testutil::run_cop_full;
        // Multi-line modifier form: report at start of expression (line 1)
        let source = b"return {\n  status: \"err\"\n}.to_json if !info\n";
        let diags = run_cop_full(&NegatedIf, source);
        assert_eq!(
            diags.len(),
            1,
            "Should flag multiline modifier: {:?}",
            diags
        );
        assert_eq!(
            diags[0].location.line, 1,
            "Should report at line 1 (start of return), not at 'if' keyword line"
        );
    }

    #[test]
    fn pattern_guard_not_flagged() {
        use crate::testutil::run_cop_full;
        let source = b"case x\nin :foo if !bar\n  nil\nend\n";
        let diags = run_cop_full(&NegatedIf, source);
        assert_eq!(
            diags.len(),
            0,
            "Should NOT flag pattern guard if: {:?}",
            diags
        );
    }

    #[test]
    fn safe_nav_chain_negation_not_flagged() {
        use crate::testutil::run_cop_full;
        let source = b"if obj&.empty?&.!\n  do_something\nend\n";
        let diags = run_cop_full(&NegatedIf, source);
        assert_eq!(
            diags.len(),
            0,
            "Should NOT flag safe-nav chain ending in &.!: {:?}",
            diags
        );
    }

    #[test]
    fn enforced_style_prefix_ignores_postfix() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("prefix".into()),
            )]),
            ..CopConfig::default()
        };
        // Postfix (modifier) form should be ignored with "prefix" style
        let source = b"do_something if !condition\n";
        let diags = run_cop_full_with_config(&NegatedIf, source, config);
        assert!(
            diags.is_empty(),
            "Should ignore modifier form with prefix style"
        );
    }

    #[test]
    fn enforced_style_prefix_flags_prefix() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("prefix".into()),
            )]),
            ..CopConfig::default()
        };
        // Prefix form should still be flagged
        let source = b"if !condition\n  do_something\nend\n";
        let diags = run_cop_full_with_config(&NegatedIf, source, config);
        assert_eq!(diags.len(), 1, "Should flag prefix form with prefix style");
    }

    #[test]
    fn enforced_style_postfix_ignores_prefix() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("postfix".into()),
            )]),
            ..CopConfig::default()
        };
        // Prefix form should be ignored with "postfix" style
        let source = b"if !condition\n  do_something\nend\n";
        let diags = run_cop_full_with_config(&NegatedIf, source, config);
        assert!(
            diags.is_empty(),
            "Should ignore prefix form with postfix style"
        );
    }

    #[test]
    fn enforced_style_postfix_flags_postfix() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("postfix".into()),
            )]),
            ..CopConfig::default()
        };
        let source = b"do_something if !condition\n";
        let diags = run_cop_full_with_config(&NegatedIf, source, config);
        assert_eq!(
            diags.len(),
            1,
            "Should flag modifier form with postfix style"
        );
    }
}
