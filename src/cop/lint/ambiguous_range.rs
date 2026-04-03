use crate::cop::shared::method_identifier_predicates;
use crate::cop::shared::node_type::{
    AND_NODE, CALL_NODE, CLASS_VARIABLE_READ_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE,
    FALSE_NODE, FLOAT_NODE, GLOBAL_VARIABLE_READ_NODE, IMAGINARY_NODE, INSTANCE_VARIABLE_READ_NODE,
    INTEGER_NODE, INTERPOLATED_STRING_NODE, LOCAL_VARIABLE_READ_NODE, NIL_NODE, OR_NODE,
    PARENTHESES_NODE, RANGE_NODE, RATIONAL_NODE, SELF_NODE, STRING_NODE, SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=27, FN=2.
///
/// FP=27: Prism surfaces array and other collection-style literals as distinct
/// node kinds, and RuboCop's `literal?` helper accepts them as unambiguous
/// range boundaries. nitrocop was only allowing a narrower basic-literal set,
/// so ranges like `[1, 0]...[1, 6]` were flagged incorrectly.
///
/// FN=2: Prism keeps `limit.times do ... end` as a `CallNode` boundary with an
/// attached `BlockNode`. RuboCop requires parentheses for that boundary because
/// the trailing block keeps the range parsing ambiguous; nitrocop was treating
/// every non-operator call boundary as acceptable.
///
/// ## Corpus investigation (2026-03-11)
///
/// FP=1, FN=0. The remaining FP was a rational literal pattern like `1/3r`
/// used as a range boundary. In Prism, `1/3r` parses as a CallNode with
/// integer receiver, method `/`, and a rational argument. The basic-literal
/// check on the receiver was rejecting it. Added `is_rational_literal()` to
/// match RuboCop's `RationalLiteral` mixin, which explicitly accepts
/// `(send (int _) :/ (rational _))` as an unambiguous boundary.
///
/// ## Corpus investigation (2026-03-11, round 2)
///
/// FP=1, FN=0. Root cause: RuboCop's `acceptable?` checks `node.begin_type?`
/// which in the Parser gem covers both parenthesized expressions `(expr)` AND
/// explicit `begin...end` blocks. nitrocop only checked `ParenthesesNode`,
/// missing `BeginNode`. A `begin; expr; end..begin; expr; end` range boundary
/// was being flagged as ambiguous. Fixed by adding `as_begin_node()` check
/// alongside `as_parentheses_node()`.
///
/// ## Corpus investigation (2026-03-11, round 3)
///
/// FP=1, FN=0. Root cause: Prism's `CallNode.block()` returns both
/// `BlockNode` (do...end / {}) AND `BlockArgumentNode` (`&:sym`, `&block`).
/// In the Parser gem, `&block` is a `block_pass` argument inside the `send`
/// node, not a wrapping block — so `call_type?` is true and it enters
/// `acceptable_call?` normally. nitrocop was blanket-rejecting any call with
/// `block().is_some()`, which incorrectly rejected `foo(&:bar)..baz(&:qux)`.
/// Fixed by only rejecting calls whose block is NOT a `BlockArgumentNode`.
///
/// ## FP fix (2026-03-21)
///
/// FP: `1.. ..1` and `1... ...1`: range nodes used as boundaries (endless to beginless).
/// RuboCop accepts `RangeNode` as an unambiguous boundary. Added `as_range_node()`
/// check to `is_acceptable_boundary`.
pub struct AmbiguousRange;

impl Cop for AmbiguousRange {
    fn name(&self) -> &'static str {
        "Lint/AmbiguousRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            AND_NODE,
            CALL_NODE,
            CLASS_VARIABLE_READ_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            FALSE_NODE,
            FLOAT_NODE,
            GLOBAL_VARIABLE_READ_NODE,
            IMAGINARY_NODE,
            INSTANCE_VARIABLE_READ_NODE,
            INTEGER_NODE,
            INTERPOLATED_STRING_NODE,
            LOCAL_VARIABLE_READ_NODE,
            NIL_NODE,
            OR_NODE,
            PARENTHESES_NODE,
            RANGE_NODE,
            RATIONAL_NODE,
            SELF_NODE,
            STRING_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
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
        let require_parens_for_chains = config.get_bool("RequireParenthesesForMethodChains", false);

        let range = match node.as_range_node() {
            Some(r) => r,
            None => return,
        };

        // Check left boundary
        if let Some(left) = range.left() {
            if !is_acceptable_boundary(&left, require_parens_for_chains) {
                let loc = left.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(
                    self.diagnostic(
                        source,
                        line,
                        column,
                        "Wrap complex range boundaries with parentheses to avoid ambiguity."
                            .to_string(),
                    ),
                );
            }
        }

        // Check right boundary
        if let Some(right) = range.right() {
            if !is_acceptable_boundary(&right, require_parens_for_chains) {
                let loc = right.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(
                    self.diagnostic(
                        source,
                        line,
                        column,
                        "Wrap complex range boundaries with parentheses to avoid ambiguity."
                            .to_string(),
                    ),
                );
            }
        }
    }
}

fn is_acceptable_boundary(node: &ruby_prism::Node<'_>, require_parens_for_chains: bool) -> bool {
    // Parenthesized expression or begin...end block
    // RuboCop's `begin_type?` covers both `(expr)` and `begin...end`.
    if node.as_parentheses_node().is_some() || node.as_begin_node().is_some() {
        return true;
    }

    // Literals: integer, float, string, symbol, nil, true, false, rational, imaginary
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
        || node.as_x_string_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
    {
        return true;
    }

    // Range nodes used as boundaries (e.g., `1.. ..1` — endless range to beginless range)
    if node.as_range_node().is_some() {
        return true;
    }

    // Variables (local, instance, class, global)
    if node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
    {
        return true;
    }

    // Constants
    if node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some() {
        return true;
    }

    // self
    if node.as_self_node().is_some() {
        return true;
    }

    // Method calls
    if let Some(call) = node.as_call_node() {
        // A trailing do...end or {} block keeps the boundary ambiguous:
        // `1..limit.times do`. But a block *argument* (`&:sym`) is just a
        // regular argument in the Parser gem (block_pass inside send), so
        // it should NOT cause rejection. Prism puts both BlockNode and
        // BlockArgumentNode in CallNode.block(); only reject actual blocks.
        if let Some(blk) = call.block() {
            if blk.as_block_argument_node().is_none() {
                return false;
            }
        }

        // Unary operations (negation, etc) are acceptable
        let name = call.name().as_slice();
        if call.receiver().is_some()
            && call.arguments().is_none()
            && (name == b"-@" || name == b"+@" || name == b"~")
        {
            return true;
        }

        // Bare method calls (no receiver) are acceptable
        if call.receiver().is_none() {
            return true;
        }

        // Rational literal pattern: `int / rational` (e.g., 1/3r) — RuboCop's
        // RationalLiteral mixin accepts these as unambiguous boundaries.
        if is_rational_literal(&call) {
            return true;
        }

        // Method calls on basic literals are NOT acceptable (e.g., 2.to_a in 1..2.to_a)
        if let Some(recv) = call.receiver() {
            if is_basic_literal(&recv) {
                return false;
            }
        }

        // Operator methods (except []) are NOT acceptable — they create
        // ambiguity like `x + 1..y - 1` where the range boundaries are unclear.
        if method_identifier_predicates::is_operator_method(name) && name != b"[]" {
            return false;
        }

        // Non-operator method calls with receiver: acceptable unless
        // RequireParenthesesForMethodChains is true.
        return !require_parens_for_chains;
    }

    // OrNode, AndNode are NOT acceptable
    if node.as_or_node().is_some() || node.as_and_node().is_some() {
        return false;
    }

    false
}

/// Matches RuboCop's `RationalLiteral` mixin: `(send (int _) :/ (rational _))`.
/// In Prism, `1/3r` parses as a CallNode with an integer receiver, method `/`,
/// and a single rational argument.
fn is_rational_literal(call: &ruby_prism::CallNode<'_>) -> bool {
    if call.name().as_slice() != b"/" {
        return false;
    }
    let Some(recv) = call.receiver() else {
        return false;
    };
    if recv.as_integer_node().is_none() {
        return false;
    }
    let Some(args) = call.arguments() else {
        return false;
    };
    let arg_list = args.arguments();
    if arg_list.len() != 1 {
        return false;
    }
    arg_list.iter().next().unwrap().as_rational_node().is_some()
}

fn is_basic_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AmbiguousRange, "cops/lint/ambiguous_range");
}
