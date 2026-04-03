//! Shared literal classification predicates, mirroring rubocop-ast's
//! `Node` class literal constants.
//!
//! Canonical source:
//! `vendor/rubocop-ast/lib/rubocop/ast/node.rb` (TRUTHY_LITERALS, FALSEY_LITERALS, etc.)
//!
//! ## Prism node type mapping
//!
//! rubocop-ast uses parser gem types; Prism splits some of these:
//! - `str` → `StringNode`
//! - `dstr` → `InterpolatedStringNode`
//! - `xstr` → `XStringNode` | `InterpolatedXStringNode`
//! - `int` → `IntegerNode`
//! - `float` → `FloatNode`
//! - `sym` → `SymbolNode`
//! - `dsym` → `InterpolatedSymbolNode`
//! - `array` → `ArrayNode`
//! - `hash` → `HashNode` (also `KeywordHashNode` for bare hash args)
//! - `regexp` → `RegularExpressionNode` | `InterpolatedRegularExpressionNode`
//! - `true` → `TrueNode`
//! - `false` → `FalseNode`
//! - `nil` → `NilNode`
//! - `irange`/`erange` → `RangeNode`
//! - `complex` → `ImaginaryNode`
//! - `rational` → `RationalNode`

/// Check if a node is a truthy literal.
///
/// Matches rubocop-ast `TRUTHY_LITERALS`:
///   `%i[str dstr xstr int float sym dsym array hash regexp true irange erange complex rational regopt]`
///
/// Note: `regopt` (regex options node) doesn't have a standalone Prism equivalent.
pub fn is_truthy_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_true_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_string_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_x_string_node().is_some()
        || node.as_interpolated_x_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
        || node.as_range_node().is_some()
}

/// Check if a node is a falsey literal (`false` or `nil`).
///
/// Matches rubocop-ast `FALSEY_LITERALS`: `%i[false nil]`
pub fn is_falsey_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_false_node().is_some() || node.as_nil_node().is_some()
}

/// Check if a node is any literal (truthy or falsey).
///
/// Matches rubocop-ast `LITERALS` (= TRUTHY_LITERALS + FALSEY_LITERALS).
pub fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    is_truthy_literal(node) || is_falsey_literal(node)
}

/// Check if a node is a composite literal (contains sub-expressions).
///
/// Matches rubocop-ast `COMPOSITE_LITERALS`:
///   `%i[dstr xstr dsym array hash irange erange regexp]`
pub fn is_composite_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_interpolated_string_node().is_some()
        || node.as_x_string_node().is_some()
        || node.as_interpolated_x_string_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_range_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
}

/// Check if a node is a basic (non-composite) literal.
///
/// Matches rubocop-ast `BASIC_LITERALS` (= LITERALS - COMPOSITE_LITERALS).
pub fn is_basic_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_regular_expression_node().is_some()
}

/// Check if a node is a mutable literal.
///
/// Matches rubocop-ast `MUTABLE_LITERALS`:
///   `%i[str dstr xstr array hash regexp irange erange]`
pub fn is_mutable_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_string_node().is_some()
        || node.as_interpolated_string_node().is_some()
        || node.as_x_string_node().is_some()
        || node.as_interpolated_x_string_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_interpolated_regular_expression_node().is_some()
        || node.as_range_node().is_some()
}

/// Check if a node is an immutable literal.
///
/// Matches rubocop-ast `IMMUTABLE_LITERALS` (= LITERALS - MUTABLE_LITERALS).
pub fn is_immutable_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_source;

    fn check_node<F: Fn(&ruby_prism::Node<'_>) -> bool>(code: &str, f: F) -> bool {
        let result = parse_source(code.as_bytes());
        let program = result.node();
        let program = program.as_program_node().unwrap();
        let stmts = program.statements();
        let node = stmts.body().iter().next().unwrap();
        f(&node)
    }

    #[test]
    fn truthy_literals() {
        assert!(check_node("42", is_truthy_literal));
        assert!(check_node("3.14", is_truthy_literal));
        assert!(check_node("'hello'", is_truthy_literal));
        assert!(check_node(":sym", is_truthy_literal));
        assert!(check_node("true", is_truthy_literal));
        assert!(check_node("[1, 2]", is_truthy_literal));
        assert!(check_node("{a: 1}", is_truthy_literal));
        assert!(check_node("/regex/", is_truthy_literal));
        assert!(check_node("1..10", is_truthy_literal));
        assert!(check_node("1r", is_truthy_literal));
        assert!(check_node("1i", is_truthy_literal));
        assert!(!check_node("false", is_truthy_literal));
        assert!(!check_node("nil", is_truthy_literal));
        assert!(!check_node("x", is_truthy_literal));
    }

    #[test]
    fn falsey_literals() {
        assert!(check_node("false", is_falsey_literal));
        assert!(check_node("nil", is_falsey_literal));
        assert!(!check_node("true", is_falsey_literal));
        assert!(!check_node("42", is_falsey_literal));
    }

    #[test]
    fn literal_covers_all() {
        assert!(check_node("42", is_literal));
        assert!(check_node("true", is_literal));
        assert!(check_node("false", is_literal));
        assert!(check_node("nil", is_literal));
        assert!(check_node("'str'", is_literal));
        assert!(check_node(":sym", is_literal));
        assert!(!check_node("x", is_literal));
    }

    #[test]
    fn basic_vs_composite() {
        assert!(check_node("42", is_basic_literal));
        assert!(check_node("'hello'", is_basic_literal));
        assert!(check_node(":sym", is_basic_literal));
        assert!(check_node("true", is_basic_literal));
        assert!(check_node("nil", is_basic_literal));
        assert!(!check_node("[1]", is_basic_literal));
        assert!(!check_node("{a: 1}", is_basic_literal));
        assert!(!check_node("1..10", is_basic_literal));

        assert!(check_node("[1]", is_composite_literal));
        assert!(check_node("{a: 1}", is_composite_literal));
        assert!(check_node("1..10", is_composite_literal));
        assert!(!check_node("42", is_composite_literal));
        assert!(!check_node("'hello'", is_composite_literal));
    }

    #[test]
    fn mutable_vs_immutable() {
        // Mutable
        assert!(check_node("'hello'", is_mutable_literal));
        assert!(check_node("[1, 2]", is_mutable_literal));
        assert!(check_node("{a: 1}", is_mutable_literal));
        assert!(check_node("/regex/", is_mutable_literal));
        assert!(check_node("1..10", is_mutable_literal));

        // Immutable
        assert!(check_node("42", is_immutable_literal));
        assert!(check_node(":sym", is_immutable_literal));
        assert!(check_node("true", is_immutable_literal));
        assert!(check_node("false", is_immutable_literal));
        assert!(check_node("nil", is_immutable_literal));

        // Cross-check
        assert!(!check_node("42", is_mutable_literal));
        assert!(!check_node("'hello'", is_immutable_literal));
    }
}
