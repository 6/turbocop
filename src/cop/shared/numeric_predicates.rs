//! Shared numeric literal predicates, mirroring rubocop-ast's `NumericNode` mixin.
//!
//! Canonical source:
//! `vendor/rubocop-ast/lib/rubocop/ast/node/mixin/numeric_node.rb`
//!
//! Provides helpers for `IntegerNode`, `FloatNode`, `RationalNode`, `ImaginaryNode`.

/// Check if a numeric literal's source has an explicit sign prefix (`+` or `-`).
///
/// Matches rubocop-ast's `NumericNode#sign?`.
///
/// ```ruby
/// +42    # true
/// -3.14  # true
/// 42     # false
/// 3.14   # false
/// ```
pub fn has_numeric_sign(source: &[u8]) -> bool {
    matches!(source.first(), Some(b'+') | Some(b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive_sign() {
        assert!(has_numeric_sign(b"+42"));
        assert!(has_numeric_sign(b"+3.14"));
        assert!(has_numeric_sign(b"+1r"));
        assert!(has_numeric_sign(b"+1i"));
    }

    #[test]
    fn test_negative_sign() {
        assert!(has_numeric_sign(b"-42"));
        assert!(has_numeric_sign(b"-3.14"));
        assert!(has_numeric_sign(b"-1r"));
        assert!(has_numeric_sign(b"-0"));
    }

    #[test]
    fn test_no_sign() {
        assert!(!has_numeric_sign(b"42"));
        assert!(!has_numeric_sign(b"3.14"));
        assert!(!has_numeric_sign(b"0"));
        assert!(!has_numeric_sign(b"1r"));
        assert!(!has_numeric_sign(b"1i"));
    }

    #[test]
    fn test_empty() {
        assert!(!has_numeric_sign(b""));
    }
}
