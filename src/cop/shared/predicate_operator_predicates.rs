//! Shared predicate operator predicates, mirroring rubocop-ast's
//! `PredicateOperatorNode` mixin.
//!
//! Canonical source:
//! `vendor/rubocop-ast/lib/rubocop/ast/node/mixin/predicate_operator_node.rb`
//!
//! Distinguishes logical operators (`&&`, `||`) from semantic/keyword operators
//! (`and`, `or`) on `AndNode` and `OrNode`.

// ---------------------------------------------------------------------------
// AndNode predicates
// ---------------------------------------------------------------------------

/// Check if an `AndNode` uses the logical operator `&&`.
///
/// ```ruby
/// a && b   # true
/// a and b  # false
/// ```
pub fn is_logical_and(node: &ruby_prism::AndNode<'_>) -> bool {
    node.operator_loc().as_slice() == b"&&"
}

/// Check if an `AndNode` uses the semantic/keyword operator `and`.
///
/// ```ruby
/// a and b  # true
/// a && b   # false
/// ```
pub fn is_semantic_and(node: &ruby_prism::AndNode<'_>) -> bool {
    node.operator_loc().as_slice() == b"and"
}

// ---------------------------------------------------------------------------
// OrNode predicates
// ---------------------------------------------------------------------------

/// Check if an `OrNode` uses the logical operator `||`.
///
/// ```ruby
/// a || b  # true
/// a or b  # false
/// ```
pub fn is_logical_or(node: &ruby_prism::OrNode<'_>) -> bool {
    node.operator_loc().as_slice() == b"||"
}

/// Check if an `OrNode` uses the semantic/keyword operator `or`.
///
/// ```ruby
/// a or b  # true
/// a || b  # false
/// ```
pub fn is_semantic_or(node: &ruby_prism::OrNode<'_>) -> bool {
    node.operator_loc().as_slice() == b"or"
}

// ---------------------------------------------------------------------------
// Generic location-based predicates
// ---------------------------------------------------------------------------

/// Check if an operator location represents a logical operator (`&&` or `||`).
///
/// Useful when you have the operator_loc from either an `AndNode` or `OrNode`
/// and want a single check.
pub fn is_logical_operator(operator_loc: &ruby_prism::Location<'_>) -> bool {
    let s = operator_loc.as_slice();
    s == b"&&" || s == b"||"
}

/// Check if an operator location represents a semantic/keyword operator (`and` or `or`).
pub fn is_semantic_operator(operator_loc: &ruby_prism::Location<'_>) -> bool {
    let s = operator_loc.as_slice();
    s == b"and" || s == b"or"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_source;

    fn check_and<F: Fn(&ruby_prism::AndNode<'_>) -> bool>(code: &str, f: F) -> bool {
        let result = parse_source(code.as_bytes());
        let program = result.node();
        let program = program.as_program_node().unwrap();
        let stmts = program.statements();
        let node = stmts.body().iter().next().unwrap();
        if let Some(and_node) = node.as_and_node() {
            f(&and_node)
        } else {
            panic!("Expected AndNode, got: {code}");
        }
    }

    fn check_or<F: Fn(&ruby_prism::OrNode<'_>) -> bool>(code: &str, f: F) -> bool {
        let result = parse_source(code.as_bytes());
        let program = result.node();
        let program = program.as_program_node().unwrap();
        let stmts = program.statements();
        let node = stmts.body().iter().next().unwrap();
        if let Some(or_node) = node.as_or_node() {
            f(&or_node)
        } else {
            panic!("Expected OrNode, got: {code}");
        }
    }

    // --- AndNode ---

    #[test]
    fn test_logical_and() {
        assert!(check_and("a && b", is_logical_and));
        assert!(!check_and("a and b", is_logical_and));
    }

    #[test]
    fn test_semantic_and() {
        assert!(check_and("a and b", is_semantic_and));
        assert!(!check_and("a && b", is_semantic_and));
    }

    // --- OrNode ---

    #[test]
    fn test_logical_or() {
        assert!(check_or("a || b", is_logical_or));
        assert!(!check_or("a or b", is_logical_or));
    }

    #[test]
    fn test_semantic_or() {
        assert!(check_or("a or b", is_semantic_or));
        assert!(!check_or("a || b", is_semantic_or));
    }

    // --- Generic ---

    #[test]
    fn test_generic_logical_operator() {
        let result = parse_source(b"a && b");
        let program = result.node().as_program_node().unwrap();
        let stmts = program.statements();
        let node = stmts.body().iter().next().unwrap();
        let and_node = node.as_and_node().unwrap();
        assert!(is_logical_operator(&and_node.operator_loc()));
        assert!(!is_semantic_operator(&and_node.operator_loc()));
    }

    #[test]
    fn test_generic_semantic_operator() {
        let result = parse_source(b"a or b");
        let program = result.node().as_program_node().unwrap();
        let stmts = program.statements();
        let node = stmts.body().iter().next().unwrap();
        let or_node = node.as_or_node().unwrap();
        assert!(is_semantic_operator(&or_node.operator_loc()));
        assert!(!is_logical_operator(&or_node.operator_loc()));
    }
}
