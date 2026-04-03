use crate::cop::shared::method_identifier_predicates;
use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for space between the name of a called method and a left parenthesis.
///
/// ## Root cause analysis (historical: 39 FP, 668 FN at 46.2% match)
///
/// **FN root cause (historical):** The `call_end > paren_end` check was meant
/// to exclude chained calls like `func (x).bar`, but also incorrectly excluded
/// calls with blocks like `func (x) { block }`. In Prism, chaining already
/// causes the first argument to NOT be a ParenthesesNode (Prism folds the
/// chain into the argument), so the check was both redundant for chains and
/// harmful for blocks. The source-text based `has_trailing_operator_or_chain`
/// check was also redundant — Prism already handles operators/chains by
/// incorporating them into the argument structure.
///
/// **FP root cause (historical):** Source-text based trailing checks were
/// incomplete. Simplified to pure AST-based approach.
///
/// ## Follow-up fix (corpus: 2 FP, 1 FN at 99.8% match)
///
/// **FP root cause:** Missing hash body exclusion. `method ({a: 1})` with
/// explicit braces produces a ParenthesesNode wrapping a HashNode. RuboCop's
/// `first_arg.hash_type?` check skips hash literals. Added hash/keyword-hash
/// body check inside the ParenthesesNode.
///
/// **FN root cause:** The compound range check was too broad. It excluded any
/// range inside parens where an endpoint was a CallNode or ParenthesesNode
/// (e.g., `rand (1.to_i..10)`). But Prism already handles true compound
/// ranges like `rand (a - b)..(c - d)` correctly — the `(a - b)` only wraps
/// the left operand, so Prism produces a RangeNode (not ParenthesesNode) as
/// the argument, and `as_parentheses_node()` filters it. Removed the compound
/// range check entirely.
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=2, FN=19.
///
/// FP=2: RuboCop only treats `.` and `&.` send syntax as candidates here, so
/// `Foo::bar (x)` constant-path calls are ignored. It also skips block-pass
/// forms like `define_method (name), &block`.
///
/// FN=19: the earlier Prism-specific hash exemption was too broad. Real-world
/// cases like `to eq ({...})` and `json.errors ({...})` still count as grouped
/// expressions in RuboCop, so parenthesized hash literals must be flagged.
///
/// All known CI FP/FN locations are fixed locally. The remaining aggregate
/// rerun delta is count-only noise within existing file-drop/parser-crash drift.
pub struct ParenthesesAsGroupedExpression;

impl Cop for ParenthesesAsGroupedExpression {
    fn name(&self) -> &'static str {
        "Lint/ParenthesesAsGroupedExpression"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must NOT have opening_loc — when there's a space before the paren,
        // Prism treats the parens as grouping (no call-level parens), so
        // opening_loc is None.
        if call.opening_loc().is_some() {
            return;
        }

        // Must have a method name
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Skip operator methods (%, +, -, ==, etc.)
        if method_identifier_predicates::is_operator_method(method_name) {
            return;
        }

        // Skip setter methods (foo=)
        if method_identifier_predicates::is_setter_method(method_name) {
            return;
        }

        // RuboCop's matcher only treats `.` and `&.` call syntax as candidates.
        // Constant-path class method calls like `Foo::bar (x)` are ignored.
        if call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"::")
        {
            return;
        }

        // Block-pass arguments (`..., &block`) are parsed separately from the
        // positional arg list. RuboCop skips these forms.
        if call
            .block()
            .is_some_and(|block| block.as_block_argument_node().is_some())
        {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();

        // Must have exactly one argument (the parenthesized expression)
        if args.len() != 1 {
            return;
        }

        let first_arg = args.iter().next().unwrap();

        // The argument must be a ParenthesesNode.
        // When Prism sees `func (x)` with space, it wraps `x` in ParenthesesNode.
        // For `func (x).bar` or `func (x) + 1`, Prism folds the chain/operator
        // into the argument, so first_arg is NOT a ParenthesesNode — those cases
        // are correctly excluded by this check.
        let paren_node = match first_arg.as_parentheses_node() {
            Some(p) => p,
            None => return,
        };

        // There must be whitespace between method name end and the `(` of the ParenthesesNode
        let msg_end = msg_loc.end_offset();
        let paren_start = paren_node.location().start_offset();

        if paren_start <= msg_end {
            return;
        }

        let between = &source.as_bytes()[msg_end..paren_start];
        if between.is_empty() || !between.iter().all(|&b| b == b' ' || b == b'\t') {
            return;
        }

        // NOTE: The compound range check (`rand (a - b)..(c - d)`) was removed.
        // Prism already handles this correctly: when the `(` only wraps the left
        // operand of a range (not the whole range), Prism does NOT wrap the argument
        // in a ParenthesesNode — it produces a RangeNode directly, which is filtered
        // out by the `as_parentheses_node()` check above. Only ranges fully wrapped
        // in parens (like `(1..10)`) reach here, and those should all be flagged.

        // Build the argument text for the message
        let paren_end = paren_node.location().end_offset();
        let arg_text = source.byte_slice(paren_start, paren_end, "(...)");

        let (line, column) = source.offset_to_line_col(paren_start);
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("`{}` interpreted as grouped expression.", arg_text),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ParenthesesAsGroupedExpression,
        "cops/lint/parentheses_as_grouped_expression"
    );

    #[test]
    fn corpus_fn_patterns() {
        // Test patterns from corpus FN analysis - symbol arguments, blocks
        let cop = ParenthesesAsGroupedExpression;
        let test_cases: &[(&[u8], usize)] = &[
            // Common FN patterns: method (:symbol)
            (b"method (:symbol)\n", 1),
            (b"method ( :all )\n", 1),
            // Inside blocks (common in RSpec)
            (b"describe do\n  subject (:all)\nend\n", 1),
            // Assignment in parens
            (b"method (var = expr)\n", 1),
            // Method call with block (was FN due to call_end > paren_end)
            (b"func (x) { |y| y }\n", 1),
            (b"func (x) do |y| y end\n", 1),
            // Should NOT detect
            (b"method(:symbol)\n", 0),
            (b"method (x).bar\n", 0),
            (b"method (x) || y\n", 0),
            (b"method (x) + 1\n", 0),
            (b"puts (2 + 3) * 4\n", 0),
            // Hash inside parens - RuboCop still treats these as grouped expressions
            (b"method ({a: 1})\n", 1),
            (b"foo ({a: 1, b: 2})\n", 1),
            (b"foo ({})\n", 1),
            // Range inside parens with call endpoints - should be flagged
            // (was FN: compound range check excluded ranges with call endpoints,
            // but Prism already handles true compound ranges by not wrapping them
            // in ParenthesesNode)
            (b"rand (1.to_i..10)\n", 1),
            (b"rand (a[0]..b[1])\n", 1),
            // Constant-path receivers and block-pass calls are accepted
            (b"if DryrunUtils::is_folder? (@url)\n  value\nend\n", 0),
            (b"define_method (test_name name), &block\n", 0),
        ];
        for (src, expected_count) in test_cases {
            let diagnostics = crate::testutil::run_cop_full(&cop, src);
            if diagnostics.len() != *expected_count {
                panic!(
                    "For {:?}: expected {} offenses, got {} ({:?})",
                    std::str::from_utf8(src).unwrap().trim(),
                    expected_count,
                    diagnostics.len(),
                    diagnostics,
                );
            }
        }
    }
}
