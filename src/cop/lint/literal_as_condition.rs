use ruby_prism::Visit;

use crate::cop::literal_predicates;
use crate::cop::node_type::{
    AND_NODE, CALL_NODE, CASE_MATCH_NODE, CASE_NODE, IF_NODE, OR_NODE, UNLESS_NODE, UNTIL_NODE,
    WHILE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for literals used as conditions or as operands in and/or expressions
/// serving as conditions of if/while/until/case-when/case-in.
///
/// ## Root causes of prior FNs (583):
/// - Missing literal types: only checked numeric/bool/nil, not string/symbol/array/hash/regex/range
/// - No ternary support (was skipping all ternaries)
/// - No modifier if/unless support
/// - No `&&`/`||` handling (truthy LHS of &&, falsey LHS of ||)
/// - No `!literal` detection
/// - No `case` without predicate (when branches with all-literal conditions)
/// - No `case_match` (pattern matching) support
/// - No recursive check through and/or/begin/! in conditions
/// - No `begin..end while`/`begin..end until` (post-loop) support
///
/// ## Root causes of prior FPs (16):
/// - Unclear without corpus data; likely edge cases now addressed by proper literal classification
///
/// ## Fixes applied:
/// - Expanded literal detection to all Ruby literal types
/// - Added truthy_literal/falsey_literal classification matching RuboCop
/// - Added on_and (truthy LHS), on_or (falsey LHS) handlers
/// - Added on_send for `!` and `not` prefix operators
/// - Added ternary and modifier if/unless support
/// - Added case without predicate and case_match support
/// - Added recursive condition checking (check_node/handle_node)
/// - Added begin..end while/until (post-loop) via is_begin_modifier()
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=20, FN=17.
///
/// FP (initial):
/// - Remaining false positives were concentrated in corpus test/spec files and pattern-matching
///   guards. The exact RuboCop suppression path was still unclear.
///
/// FN (fixed):
/// - Opal-style backtick JavaScript with interpolation parses as `InterpolatedXStringNode`, not
///   `XStringNode`. Treating only plain xstrings as literals missed conditions like
///   ``if `#{value}``` and ``while `#{counter} < 10``.
///
/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=20, FN=0.
///
/// Previous fix: Removed `XStringNode` and `InterpolatedXStringNode` from `is_literal()`.
/// This was based on an incorrect claim that rubocop-ast's `literal?` excludes `xstr`.
/// In fact, `xstr` IS in rubocop-ast's `TRUTHY_LITERALS`. Empirically verified with
/// RuboCop v1.85.1: `if \`cmd\`` IS flagged as an offense. The removal was wrong.
///
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=20, FN=127.
///
/// FN fix: Added `XStringNode` and `InterpolatedXStringNode` back to `is_literal()`.
/// Empirically verified all literal types against RuboCop v1.85.1 with Prism parser:
/// - Flagged in conditions: int, float, str, dstr, sym, dsym, array, hash, true, false,
///   nil, rational, complex, xstr (both plain and interpolated)
/// - NOT flagged: regex in if/while/unless/until (Prism converts to MatchLastLineNode),
///   range in if (Prism converts to FlipFlopNode)
/// - `RegularExpressionNode`/`InterpolatedRegularExpressionNode` correctly remain in
///   `is_literal()` — they appear as `case` predicates and `when` conditions where
///   RuboCop does flag them. Only in if/while/unless/until do regexes become
///   MatchLastLineNode/InterpolatedMatchLastLineNode (not in our literal set).
///
/// Also added test coverage for: xstring in if/while/bang contexts, elsif with literal
/// condition, interpolated symbol (`:"#{a}"`), and regex/interpolated-regex as
/// no-offense in if conditions.
///
/// FP=20 root cause analysis:
/// - ~10 FPs: Pattern matching guards (`in X if true`, `in X if false`) in case/in
///   blocks were incorrectly flagged. Prism represents these guards as IfNode/UnlessNode
///   inside InNode.pattern, so the AST walker visits them and the IfNode handler fires.
///   RuboCop does NOT fire `on_if` for pattern matching guards (in the Parser gem AST,
///   guards are part of the `in_pattern` node, not separate `if` nodes).
///   Fix: Added `is_pattern_matching_guard()` check that detects when an IfNode/UnlessNode
///   is on a line starting with `in ` (indicating it's a guard, not a standalone condition).
/// - ~10 FPs: Config/exclude differences (files excluded by RuboCop project config but not
///   by nitrocop). These are config resolution issues, not cop logic bugs.
///
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=10, FN=0. All remaining FPs are config/exclude
/// differences — files excluded by target project .rubocop.yml but not by
/// nitrocop's config resolution. No cop logic bugs remain.
///
/// ## Corpus investigation (2026-03-15)
///
/// The remaining FP=10 were not config noise. The corpus oracle runs against
/// `bench/corpus/baseline_rubocop.yml`, so repo-local excludes are ignored.
///
/// Root cause:
/// - RuboCop 1.84.2 crashes and emits no offense when `if`/`unless` has a
///   literal condition plus an explicit `else` branch whose body is empty, e.g.
///   `if false; 123; else; end`, `if true; else; end`, `unless 1; 2; else; end`.
/// - The same crash shape appears on `elsif` because Prism models it as a
///   nested `IfNode` with an empty explicit `else`.
///
/// Fix:
/// - Skip literal-condition offenses for `IfNode`/`UnlessNode` when the explicit
///   `else` branch exists and its statements are empty. This matches the corpus
///   oracle's RuboCop behavior.
///
/// ## Corpus investigation (2026-03-18)
///
/// Corpus oracle reported FP=0, FN=11.
///
/// FN root cause: The `if_has_empty_else` skip was too aggressive. It skipped ALL
/// cases where the else branch was empty, including `if true; <nested>; else; end`
/// where the then-body is non-empty. RuboCop 1.84.2 only crashes when BOTH the
/// then-body AND else-body are empty (e.g. `if true; else; end`).
/// When the then-body has content, RuboCop correctly flags the literal.
///
/// Verified from corpus: jruby's `bench/compiler/bench_compilation.rb` has 10
/// nested `if true;` each with its own `else; end`. The 9 outer levels have
/// non-empty then-bodies (nested ifs) → RuboCop flags them. The innermost has
/// empty then and empty else → RuboCop crashes/skips. rufo has `if 1; 2; else; end`
/// (non-empty then, empty else) → RuboCop flags.
///
/// Fix: Changed `if_has_empty_else` to `if_has_empty_body_and_empty_else`, which
/// only skips when both branches are empty. Verified with verify-cop-locations.py:
/// all 11 FN fixed, 0 new FP.
///
/// ## Corpus investigation (2026-03-18, second pass)
///
/// Corpus oracle reported FP=4, FN=0.
///
/// FP root cause: The `if_has_empty_body_and_empty_else` check was still not
/// precise enough. RuboCop's `correct_if_node` crashes when it reaches the
/// `node.else? || node.ternary?` branch and calls `node.else_branch.source`
/// on nil (empty else body). This path is reached when `condition_evaluation?`
/// returns false:
/// - For `if`: condition is falsey → result=false → crashes on nil else_branch
/// - For `unless`: condition is truthy → result=false → crashes on nil else_branch
///
/// When `condition_evaluation?` returns true, the corrector uses `if_branch`
/// (non-nil when then-body has content), so no crash occurs.
///
/// Examples: `if false; 123; else; end` (falsey in if → crash → no offense),
/// `unless 1; 2; else; end` (truthy in unless → crash → no offense),
/// but `if 1; 2; else; end` (truthy in if → uses if_branch → offense reported).
///
/// Fix: Replaced `if_has_empty_body_and_empty_else` with
/// `if_should_skip_for_empty_else` / `unless_should_skip_for_empty_else`
/// which account for the truthiness of the condition.
pub struct LiteralAsCondition;

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    literal_predicates::is_literal(node)
}

fn is_truthy_literal(node: &ruby_prism::Node<'_>) -> bool {
    literal_predicates::is_truthy_literal(node)
}

fn is_falsey_literal(node: &ruby_prism::Node<'_>) -> bool {
    literal_predicates::is_falsey_literal(node)
}

/// Check if an array node contains only primitive (basic) literals recursively.
fn is_primitive_array(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(array) = node.as_array_node() {
        array.elements().iter().all(|elem| {
            if elem.as_array_node().is_some() {
                is_primitive_array(&elem)
            } else {
                literal_predicates::is_basic_literal(&elem)
            }
        })
    } else {
        false
    }
}

/// Check and report a literal inside a `!` or `not()` receiver.
/// Unwraps one level of parentheses. Does NOT recurse into compound
/// expressions (and/or) since those are handled by on_and/on_or.
fn check_bang_receiver(
    cop: &LiteralAsCondition,
    source: &SourceFile,
    recv: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if is_literal(recv) {
        add_literal_offense(cop, source, recv, diagnostics);
        return;
    }
    // Unwrap parentheses: `!(expr)` or `not(expr)`
    if let Some(parens) = recv.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                let body_nodes: Vec<_> = stmts.body().iter().collect();
                if body_nodes.len() == 1 && is_literal(&body_nodes[0]) {
                    add_literal_offense(cop, source, &body_nodes[0], diagnostics);
                }
            }
        }
    }
}

/// Check if an IfNode or UnlessNode is a pattern matching guard (e.g., `in 4 if true`).
/// In Prism, pattern matching guards are represented as IfNode/UnlessNode inside
/// InNode.pattern. We detect this by checking if the text from line start to the
/// node's start offset is `in ` (with optional leading whitespace).
fn is_pattern_matching_guard(source: &SourceFile, node: &ruby_prism::Node<'_>) -> bool {
    let loc = node.location();
    let start = loc.start_offset();
    let (line, _col) = source.offset_to_line_col(start);
    if let Some(line_start) = source.line_col_to_offset(line, 0) {
        if let Some(prefix) = source.try_byte_slice(line_start, start) {
            let trimmed = prefix.trim();
            return trimmed == "in";
        }
    }
    false
}

fn node_source_text<'a>(node: &ruby_prism::Node<'a>) -> &'a str {
    let loc = node.location();
    std::str::from_utf8(loc.as_slice()).unwrap_or("literal")
}

fn statements_are_empty(statements: Option<ruby_prism::StatementsNode<'_>>) -> bool {
    match statements {
        Some(stmts) => stmts.body().iter().next().is_none(),
        None => true,
    }
}

/// RuboCop 1.84.2's `correct_if_node` crashes when it tries to access
/// `node.else_branch.source` on a nil else_branch. This happens when:
/// - The else clause exists but its body is empty (`else_branch` is nil)
/// - `condition_evaluation?` returns false, causing the corrector to reach
///   the `node.else? || node.ternary?` branch and call `.source` on nil
///
/// `condition_evaluation?` returns false when:
/// - For `if`: the condition is falsey (not truthy)
/// - For `unless`: the condition is truthy (not falsey)
///
/// When both branches are empty, the crash always occurs regardless of
/// condition truthiness, because all corrector paths fail.
fn if_should_skip_for_empty_else(
    if_node: &ruby_prism::IfNode<'_>,
    predicate: &ruby_prism::Node<'_>,
) -> bool {
    let else_is_empty = if let Some(subsequent) = if_node.subsequent() {
        if let Some(else_node) = subsequent.as_else_node() {
            statements_are_empty(else_node.statements())
        } else {
            false
        }
    } else {
        false
    };

    if !else_is_empty {
        return false;
    }

    let body_empty = statements_are_empty(if_node.statements());
    if body_empty {
        // Both branches empty → crash regardless of condition
        return true;
    }

    // Non-empty body, empty else: crash only when condition_evaluation? returns false
    // For `if`: result = cond.truthy_literal? → false when cond is falsey
    is_falsey_literal(predicate)
}

fn unless_should_skip_for_empty_else(
    unless_node: &ruby_prism::UnlessNode<'_>,
    predicate: &ruby_prism::Node<'_>,
) -> bool {
    let else_is_empty = if let Some(else_node) = unless_node.else_clause() {
        statements_are_empty(else_node.statements())
    } else {
        false
    };

    if !else_is_empty {
        return false;
    }

    let body_empty = statements_are_empty(unless_node.statements());
    if body_empty {
        // Both branches empty → crash regardless of condition
        return true;
    }

    // Non-empty body, empty else: crash only when condition_evaluation? returns false
    // For `unless`: result = cond.falsey_literal? → false when cond is truthy
    is_truthy_literal(predicate)
}

fn add_literal_offense(
    cop: &LiteralAsCondition,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let loc = node.location();
    let literal_text = node_source_text(node);
    let (line, column) = source.offset_to_line_col(loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!("Literal `{literal_text}` appeared as a condition."),
    ));
}

impl Cop for LiteralAsCondition {
    fn name(&self) -> &'static str {
        "Lint/LiteralAsCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            AND_NODE,
            CALL_NODE,
            CASE_MATCH_NODE,
            CASE_NODE,
            IF_NODE,
            OR_NODE,
            UNLESS_NODE,
            UNTIL_NODE,
            WHILE_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // on_and: truthy literal on LHS of &&
        if let Some(and_node) = node.as_and_node() {
            let lhs = and_node.left();
            if is_truthy_literal(&lhs) {
                add_literal_offense(self, source, &lhs, diagnostics);
            }
            return;
        }

        // on_or: falsey literal on LHS of ||
        if let Some(or_node) = node.as_or_node() {
            let lhs = or_node.left();
            if is_falsey_literal(&lhs) {
                add_literal_offense(self, source, &lhs, diagnostics);
            }
            return;
        }

        // on_send: ! and not() operators
        // Only flags direct literal receivers. Nested &&/||/! within the receiver
        // are handled by on_and/on_or and recursive on_send calls on inner CallNodes.
        if let Some(call) = node.as_call_node() {
            let name = call.name();
            if name.as_slice() == b"!" {
                // This handles both `!expr` (prefix bang) and `not(expr)`
                // In Prism, `not(expr)` is also a CallNode with name `!`
                if let Some(recv) = call.receiver() {
                    check_bang_receiver(self, source, &recv, diagnostics);
                }
            }
            return;
        }

        // on_if: if/unless/elsif/ternary with literal condition
        // Only checks the direct predicate -- nested &&/||/! are handled by
        // on_and/on_or/on_send handlers respectively.
        if let Some(if_node) = node.as_if_node() {
            // Skip pattern matching guards (e.g., `in 4 if true`)
            if is_pattern_matching_guard(source, node) {
                return;
            }
            let predicate = if_node.predicate();
            if if_should_skip_for_empty_else(&if_node, &predicate) {
                return;
            }

            if is_falsey_literal(&predicate) || is_truthy_literal(&predicate) {
                add_literal_offense(self, source, &predicate, diagnostics);
            }
            return;
        }

        // on_unless: unless with literal condition (UnlessNode is separate in Prism)
        if let Some(unless_node) = node.as_unless_node() {
            // Skip pattern matching guards (e.g., `in 4 unless false`)
            if is_pattern_matching_guard(source, node) {
                return;
            }
            let predicate = unless_node.predicate();
            if unless_should_skip_for_empty_else(&unless_node, &predicate) {
                return;
            }
            if is_falsey_literal(&predicate) || is_truthy_literal(&predicate) {
                add_literal_offense(self, source, &predicate, diagnostics);
            }
            return;
        }

        // on_while (includes begin..end while via is_begin_modifier)
        if let Some(while_node) = node.as_while_node() {
            let predicate = while_node.predicate();
            let pred_text = node_source_text(&predicate);

            // RuboCop skips `while true` (common infinite loop idiom)
            if pred_text == "true" {
                return;
            }

            if is_literal(&predicate) {
                add_literal_offense(self, source, &predicate, diagnostics);
            }
            return;
        }

        // on_until (includes begin..end until via is_begin_modifier)
        if let Some(until_node) = node.as_until_node() {
            let predicate = until_node.predicate();
            let pred_text = node_source_text(&predicate);

            // RuboCop skips `until false` (common infinite loop idiom)
            if pred_text == "false" {
                return;
            }

            if is_literal(&predicate) {
                add_literal_offense(self, source, &predicate, diagnostics);
            }
            return;
        }

        // on_case
        if let Some(case_node) = node.as_case_node() {
            if let Some(predicate) = case_node.predicate() {
                // Case with predicate: check if predicate is literal
                // Skip non-primitive arrays and interpolated strings
                if predicate.as_array_node().is_some() && !is_primitive_array(&predicate) {
                    return;
                }
                if predicate.as_interpolated_string_node().is_some() {
                    return;
                }

                // Only flag direct literals. Nested &&/||/! are handled by
                // on_and/on_or/on_send. RuboCop's check_case only recurses through
                // keyword `and`/`or` (operator_keyword?), not &&/||, which is rare.
                if is_falsey_literal(&predicate) || is_truthy_literal(&predicate) {
                    add_literal_offense(self, source, &predicate, diagnostics);
                }
            } else {
                // Case without predicate: check when branches
                for condition in case_node.conditions().iter() {
                    if let Some(when_node) = condition.as_when_node() {
                        let conditions: Vec<_> = when_node.conditions().iter().collect();
                        if conditions.iter().all(|c| is_literal(c)) {
                            // Report on the range of all conditions
                            // For simplicity, report on each condition individually
                            // RuboCop reports on the combined range of all conditions
                            if conditions.len() == 1 {
                                add_literal_offense(self, source, &conditions[0], diagnostics);
                            } else {
                                // Report on the combined range from first to last condition
                                let first = conditions.first().unwrap();
                                let last = conditions.last().unwrap();
                                let first_loc = first.location();
                                let last_loc = last.location();
                                let (line, column) =
                                    source.offset_to_line_col(first_loc.start_offset());
                                let start = first_loc.start_offset();
                                let end = last_loc.start_offset() + last_loc.as_slice().len();
                                let combined_text = source.byte_slice(start, end, "literal");
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!("Literal `{combined_text}` appeared as a condition."),
                                ));
                            }
                        }
                    }
                }
            }
            return;
        }

        // on_case_match (pattern matching)
        if let Some(case_match) = node.as_case_match_node() {
            if let Some(predicate) = case_match.predicate() {
                // Check if any descendant is a match variable - if so, skip
                // (it's being used as a pattern matching expression)
                if has_match_var_descendant(&case_match) {
                    return;
                }

                // Skip non-primitive arrays and interpolated strings
                if predicate.as_array_node().is_some() && !is_primitive_array(&predicate) {
                    return;
                }
                if predicate.as_interpolated_string_node().is_some() {
                    return;
                }

                if is_literal(&predicate) {
                    add_literal_offense(self, source, &predicate, diagnostics);
                }
            } else {
                // case/in without predicate: check in_pattern branches
                for condition in case_match.conditions().iter() {
                    if let Some(in_node) = condition.as_in_node() {
                        let pattern = in_node.pattern();
                        if is_literal(&pattern) {
                            // Report on the in_node itself (matching RuboCop behavior)
                            let loc = condition.location();
                            let text = node_source_text(&condition);
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!("Literal `{text}` appeared as a condition."),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Visitor that checks if any descendant is a match variable (LocalVariableTargetNode).
struct MatchVarFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for MatchVarFinder {
    fn visit_local_variable_target_node(
        &mut self,
        _node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        self.found = true;
    }

    fn visit_capture_pattern_node(&mut self, _node: &ruby_prism::CapturePatternNode<'pr>) {
        self.found = true;
    }
}

/// Check if any descendant of a case_match node is a match variable.
fn has_match_var_descendant(node: &ruby_prism::CaseMatchNode<'_>) -> bool {
    let mut finder = MatchVarFinder { found: false };
    for cond in node.conditions().iter() {
        if let Some(in_node) = cond.as_in_node() {
            finder.visit(&in_node.pattern());
            if finder.found {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LiteralAsCondition, "cops/lint/literal_as_condition");

    #[test]
    fn test_if_true_semicolon() {
        let cop = LiteralAsCondition;
        let diags = crate::testutil::run_cop_full(&cop, b"if true;\n  x = 1\nend\n");
        assert!(
            !diags.is_empty(),
            "should detect literal in 'if true;' but got no diagnostics"
        );
    }

    #[test]
    fn test_nested_if_true_semicolons() {
        let cop = LiteralAsCondition;
        let src = b"if true;\n  if true;\n    x = 1\n  end\nend\n";
        let diags = crate::testutil::run_cop_full(&cop, src);
        assert_eq!(
            diags.len(),
            2,
            "should detect 2 literals in nested 'if true;'"
        );
    }

    #[test]
    fn test_if_literal_semicolon_else_end_offense() {
        // RuboCop flags literal condition even with empty else when then-body is non-empty
        let cop = LiteralAsCondition;
        let diags = crate::testutil::run_cop_full(&cop, b"if 1; 2; else; end\n");
        assert_eq!(
            diags.len(),
            1,
            "should detect literal in 'if 1; 2; else; end' but got: {:?}",
            diags
        );
    }

    #[test]
    fn test_if_literal_both_empty_no_offense() {
        // RuboCop 1.84.2 crashes when both then-body and else-body are empty
        let cop = LiteralAsCondition;
        let diags = crate::testutil::run_cop_full(&cop, b"if true; else; end\n");
        assert!(
            diags.is_empty(),
            "should NOT detect literal in 'if true; else; end' (both empty) but got: {:?}",
            diags
        );
    }

    #[test]
    fn test_jruby_nested_if_true_with_empty_else_on_all_levels() {
        // jruby corpus: 10 nested `if true;` each with its own empty else.
        // The 9 outer ones have non-empty then-bodies → RuboCop flags them.
        // The innermost has both empty then and empty else → RuboCop crashes/skips.
        let cop = LiteralAsCondition;
        let src = b"\
if true;\n\
  if true;\n\
    if true;\n\
      if true;\n\
        if true;\n\
          if true;\n\
            if true;\n\
              if true;\n\
                if true;\n\
                  if true;\n\
                  else\n\
                  end\n\
                else\n\
                end\n\
              else\n\
              end\n\
            else\n\
            end\n\
          else\n\
          end\n\
        else\n\
        end\n\
      else\n\
      end\n\
    else\n\
    end\n\
  else\n\
  end\n\
else\n\
end\n";
        let diags = crate::testutil::run_cop_full(&cop, src);
        assert_eq!(
            diags.len(),
            9,
            "expected 9 offenses (outer nested if true; with non-empty then) but got {}: {:?}",
            diags.len(),
            diags
        );
    }
}
