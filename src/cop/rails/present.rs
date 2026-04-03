use crate::cop::shared::node_type::{AND_NODE, CALL_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/Present cop — suggests using `present?` instead of `!blank?`, `unless blank?`,
/// or `!nil? && !empty?`.
///
/// ## Investigation (2026-03-08)
///
/// **FP root cause (84 FP):** `check_unless_blank` flagged `unless foo.blank? ... else ... end`.
/// RuboCop skips these when `Style/UnlessElse` is enabled (default), because the `unless/else`
/// form should be rewritten by `Style/UnlessElse` first. Fix: skip when `else_clause` is present.
///
/// **FN root cause (19 FN):** `check_not_nil_and_not_empty` only matched `!foo.nil?` and bare
/// `foo` on the left side. RuboCop's `exists_and_not_empty?` also matches:
/// - `foo != nil && !foo.empty?` (inequality nil check)
/// - `!!foo && !foo.empty?` (double negation)
///
/// Fix: added patterns 3 and 4 for these forms.
///
/// ## Investigation (2026-03-10)
///
/// **FP root cause (68 FP):** Two issues:
/// 1. Safe navigation (`&.`) not excluded. RuboCop's NodePatterns use `send` which does not
///    match `csend` (safe navigation). So `!foo&.blank?`, `unless foo&.blank?`, and
///    `!foo&.empty?` should NOT be flagged. Nitrocop was matching all CallNodes regardless
///    of `call_operator_loc()`.
/// 2. Receiver mismatch in `NotNilAndNotEmpty` patterns 1, 3, 4. RuboCop requires `var1 == var2`
///    (same receiver on both sides of `&&`). Nitrocop only checked source equality in pattern 2
///    but not in patterns 1 (`!x.nil? && !y.empty?`), 3 (`x != nil && !y.empty?`), or
///    4 (`!!x && !y.empty?`).
///
/// Fix: Added `is_safe_nav()` helper to skip safe navigation calls in all three check methods.
/// Added receiver source comparison for patterns 1, 3, and 4.
///
/// ## Investigation (2026-03-14)
///
/// **FP root cause (57 FP):** `blank?` called WITH arguments was flagged. RuboCop's NodePattern
/// `(send $_ :blank?)` only matches when `blank?` has NO arguments. The pattern
/// `!Helpers.blank?(value)` or `unless Helpers.blank?(value)` uses `blank?` as a class method
/// with an argument — RuboCop doesn't flag these, nitrocop did.
///
/// Fix: Added argument count check in `check_not_blank` and `check_unless_blank`. If the `blank?`
/// call has any arguments, skip flagging to match RuboCop's NodePattern behavior.
///
/// ## Investigation (2026-03-15): FP=14, FN=14
///
/// **Root cause:** Location mismatch for modifier `unless`. For `x unless y.blank?`, nitrocop
/// was reporting at the start of the entire expression (`x`), while RuboCop reports at the
/// `unless` keyword. For multiline lambdas like `-> { ... } unless x.blank?`, this caused
/// FP at the lambda's start line and FN at the `unless` keyword line.
///
/// Fix: Use `unless_node.keyword_loc()` instead of `node.location()` so the offense is always
/// reported at the `unless` keyword. Also updated message to include the actual receiver and
/// predicate source text (matching RuboCop's dynamic message format).
///
/// ## Investigation (2026-03-26)
///
/// **FN triage result:** Representative corpus snippets like
/// `add_comment(comment) if comment && ! comment.empty?` and
/// `if comment and not comment.empty? then` already produce offenses when this cop is invoked
/// directly through `run_cop_full`. The matcher handles the old `and`/`not` forms and modifier
/// control-flow wrappers correctly.
///
/// **Actual blocker:** The remaining corpus miss is runtime/plugin activation, not matcher logic.
/// `Rails` is treated as a plugin department, so CLI runs disable `Rails/Present` unless
/// `rubocop-rails` is considered loaded via config `require:` resolution. `--force-default-config`
/// and temp overlay configs rooted outside the repo can leave `require_departments` empty or make
/// `bundle info --path rubocop-rails` run from `/tmp`, so the cop never executes even though the
/// AST matcher is correct.
///
/// **Dead end avoided:** expanding the fixtures and changing offense text did not affect the real
/// FN, because the direct cop reproduction already passed before any matcher change.
///
/// **Correct fix would need:** config/plugin loading work in `src/config/mod.rs` or overlay
/// working-directory handling, outside this cop.
pub struct Present;

impl Cop for Present {
    fn name(&self) -> &'static str {
        "Rails/Present"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, CALL_NODE, UNLESS_NODE]
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
        let not_nil_and_not_empty = config.get_bool("NotNilAndNotEmpty", true);
        let not_blank = config.get_bool("NotBlank", true);
        let unless_blank = config.get_bool("UnlessBlank", true);

        // Check for `unless foo.blank?` => `if foo.present?` (UnlessBlank)
        if unless_blank {
            if let Some(diag) = self.check_unless_blank(source, node) {
                diagnostics.push(diag);
            }
        }

        // Check for `!nil? && !empty?` => `present?` (NotNilAndNotEmpty)
        if not_nil_and_not_empty {
            if let Some(diag) = self.check_not_nil_and_not_empty(source, node) {
                diagnostics.push(diag);
            }
        }

        // Check for `!blank?` => `present?` (NotBlank)
        if !not_blank {
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"!" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"blank?" {
            return;
        }

        // Skip safe navigation: !foo&.blank? — RuboCop's `send` doesn't match `csend`
        if is_safe_nav(&inner_call) {
            return;
        }

        // RuboCop's NodePattern `(send $_ :blank?)` only matches blank? with NO arguments.
        // `!Helpers.blank?(value)` (blank? called as class method with arg) must not be flagged.
        if inner_call
            .arguments()
            .is_some_and(|a| !a.arguments().is_empty())
        {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `present?` instead of `!blank?`.".to_string(),
        ));
    }
}

/// Returns true if the given CallNode uses safe navigation (`&.`).
fn is_safe_nav(call: &ruby_prism::CallNode<'_>) -> bool {
    call.call_operator_loc()
        .is_some_and(|op| op.as_slice() == b"&.")
}

impl Present {
    /// Check for `unless foo.blank?` pattern.
    /// Skips `unless foo.blank? ... else ... end` — RuboCop defers to Style/UnlessElse for those.
    fn check_unless_blank(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Option<Diagnostic> {
        let unless_node = node.as_unless_node()?;

        // RuboCop skips unless/else when Style/UnlessElse is enabled (default).
        // Conservative: always skip when else clause is present.
        if unless_node.else_clause().is_some() {
            return None;
        }

        // Predicate should be `foo.blank?` (not safe navigation `foo&.blank?`)
        let predicate = unless_node.predicate();
        let pred_call = predicate.as_call_node()?;
        if pred_call.name().as_slice() != b"blank?" {
            return None;
        }
        if is_safe_nav(&pred_call) {
            return None;
        }
        // RuboCop's NodePattern `(send $_ :blank?)` only matches blank? with NO arguments.
        // `unless Helpers.blank?(value)` should NOT be flagged.
        if pred_call
            .arguments()
            .is_some_and(|a| !a.arguments().is_empty())
        {
            return None;
        }

        // RuboCop reports offense at the `unless` keyword for modifier form,
        // or from the start of the block form. Using keyword_loc() covers both:
        // for modifier `x unless y.blank?`, this is the `unless` keyword (not `x`).
        let kw_loc = unless_node.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        // Build message like RuboCop: `Use `if <recv>.present?` instead of `unless <recv>.blank?`.`
        let bytes = source.as_bytes();
        let prefer = if let Some(recv) = pred_call.receiver() {
            let recv_src = &bytes[recv.location().start_offset()..recv.location().end_offset()];
            format!("{}.present?", std::str::from_utf8(recv_src).unwrap_or(""))
        } else {
            "present?".to_string()
        };
        // `current` = source from `unless` keyword to end of blank? predicate
        let current_end = predicate.location().end_offset();
        let current_src = &bytes[kw_loc.start_offset()..current_end];
        let current = std::str::from_utf8(current_src).unwrap_or("unless blank?");
        Some(self.diagnostic(
            source,
            line,
            column,
            format!("Use `if {prefer}` instead of `{current}`."),
        ))
    }

    /// Check for `!foo.nil? && !foo.empty?` or `foo && !foo.empty?` pattern.
    fn check_not_nil_and_not_empty(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
    ) -> Option<Diagnostic> {
        let and_node = node.as_and_node()?;

        let left = and_node.left();
        let right = and_node.right();

        // Right must be: !foo.empty? (call to ! on empty?, not safe navigation)
        let right_not = right.as_call_node()?;
        if right_not.name().as_slice() != b"!" {
            return None;
        }
        let right_inner = right_not.receiver()?;
        let right_pred = right_inner.as_call_node()?;
        if right_pred.name().as_slice() != b"empty?" {
            return None;
        }
        // Skip safe navigation: !foo&.empty? — RuboCop's `send` doesn't match `csend`
        if is_safe_nav(&right_pred) {
            return None;
        }

        // Helper: get the receiver source of the right-side empty? call
        let right_recv_src = right_pred
            .receiver()
            .map(|r| &source.as_bytes()[r.location().start_offset()..r.location().end_offset()]);

        // Helper: check if left-side receiver matches right-side receiver
        let receivers_match = |left_recv: Option<ruby_prism::Node<'_>>| -> bool {
            match (left_recv, right_recv_src) {
                (Some(lr), Some(rr_src)) => {
                    let lr_src = &source.as_bytes()
                        [lr.location().start_offset()..lr.location().end_offset()];
                    lr_src == rr_src
                }
                (None, None) => true,
                _ => false,
            }
        };

        // Pattern 1: Left is !foo.nil? (explicit nil check)
        let matches = if let Some(left_not) = left.as_call_node() {
            if left_not.name().as_slice() == b"!" {
                if let Some(left_inner) = left_not.receiver() {
                    if let Some(left_pred) = left_inner.as_call_node() {
                        left_pred.name().as_slice() == b"nil?"
                            && receivers_match(left_pred.receiver())
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        // Pattern 2: Left is foo (implicit nil check: foo && !foo.empty?)
        // The left side is any expression and right side is !<same_expr>.empty?
        let matches = matches || {
            let left_src =
                &source.as_bytes()[left.location().start_offset()..left.location().end_offset()];
            if let Some(rr_src) = right_recv_src {
                left_src == rr_src
            } else {
                false
            }
        };

        // Pattern 3: Left is foo != nil (inequality nil check)
        let matches = matches || {
            if let Some(left_call) = left.as_call_node() {
                if left_call.name().as_slice() == b"!=" {
                    if let Some(args) = left_call.arguments() {
                        let arg_list = args.arguments();
                        arg_list.len() == 1
                            && arg_list.first().is_some_and(|a| a.as_nil_node().is_some())
                            && receivers_match(left_call.receiver())
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };

        // Pattern 4: Left is !!foo (double negation)
        let matches = matches || {
            if let Some(outer_not) = left.as_call_node() {
                if outer_not.name().as_slice() == b"!" {
                    if let Some(inner) = outer_not.receiver() {
                        if let Some(inner_not) = inner.as_call_node() {
                            if inner_not.name().as_slice() == b"!" {
                                receivers_match(inner_not.receiver())
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };

        if !matches {
            return None;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        Some(self.diagnostic(
            source,
            line,
            column,
            "Use `present?` instead of `!nil? && !empty?`.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Present, "cops/rails/present");

    #[test]
    fn not_blank_false_skips_bang_blank() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("NotBlank".to_string(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let source = b"!x.blank?\n";
        assert_cop_no_offenses_full_with_config(&Present, source, config);
    }

    #[test]
    fn not_nil_and_not_empty_false_skips_pattern() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "NotNilAndNotEmpty".to_string(),
                serde_yml::Value::Bool(false),
            )]),
            ..CopConfig::default()
        };
        let source = b"!foo.nil? && !foo.empty?\n";
        assert_cop_no_offenses_full_with_config(&Present, source, config);
    }

    #[test]
    fn unless_blank_false_skips_unless() {
        use crate::cop::CopConfig;
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("UnlessBlank".to_string(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let source = b"unless x.blank?\n  do_something\nend\n";
        assert_cop_no_offenses_full_with_config(&Present, source, config);
    }

    #[test]
    fn detects_modifier_if_exists_and_not_empty() {
        use crate::testutil::run_cop_full;

        let source =
            b"comment = description.strip\nadd_comment(comment) if comment && ! comment.empty?\n";
        let diagnostics = run_cop_full(&Present, source);

        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        let diag = &diagnostics[0];
        assert_eq!(diag.location.line, 2);
        assert_eq!(diag.location.column, 24);
        assert_eq!(
            diag.message,
            "Use `present?` instead of `!nil? && !empty?`."
        );
    }
}
