//! Shared method identifier predicates, mirroring rubocop-ast's
//! `MethodIdentifierPredicates` module.
//!
//! Canonical source:
//! `vendor/rubocop-ast/lib/rubocop/ast/node/mixin/method_identifier_predicates.rb`

/// The 28 canonical Ruby operator methods.
///
/// From rubocop-ast `OPERATOR_METHODS`:
///   `%i[| ^ & <=> == === =~ > >= < <= << >> + - * / % ** ~ +@ -@ !@ ~@ [] []= ! != !~ \`]`
pub const OPERATOR_METHODS: &[&[u8]] = &[
    b"|", b"^", b"&", b"<=>", b"==", b"===", b"=~", b">", b">=", b"<", b"<=", b"<<", b">>", b"+",
    b"-", b"*", b"/", b"%", b"**", b"~", b"+@", b"-@", b"!@", b"~@", b"[]", b"[]=", b"!", b"!=",
    b"!~", b"`",
];

/// The 7 canonical comparison operators.
///
/// From rubocop-ast `Node::COMPARISON_OPERATORS`:
///   `%i[== === != <= >= > <]`
pub const COMPARISON_OPERATORS: &[&[u8]] = &[b"==", b"===", b"!=", b"<=", b">=", b">", b"<"];

/// Check if a method name is one of the 28 canonical operator methods.
pub fn is_operator_method(name: &[u8]) -> bool {
    OPERATOR_METHODS.contains(&name)
}

/// Check if a method name is a setter method.
///
/// A setter method ends with `=` but is NOT a comparison operator and NOT `!=`.
/// Matches rubocop-ast's `assignment_method?`:
///   `!comparison_method? && method_name.to_s.end_with?('=')`
pub fn is_setter_method(name: &[u8]) -> bool {
    name.ends_with(b"=") && !is_comparison_method(name)
}

/// Check if a method name is one of the 7 comparison operators.
pub fn is_comparison_method(name: &[u8]) -> bool {
    COMPARISON_OPERATORS.contains(&name)
}

/// Check if a method name is an assignment method (same as `is_setter_method`
/// for name-based checks).
///
/// Matches rubocop-ast's `assignment_method?`.
pub fn is_assignment_method(name: &[u8]) -> bool {
    is_setter_method(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_methods_count() {
        // 28 operators + != + !~ + ` = 30 entries
        // Actually: | ^ & <=> == === =~ > >= < <= << >> + - * / % ** ~ +@ -@ !@ ~@ [] []= ! != !~ ` = 30
        assert_eq!(OPERATOR_METHODS.len(), 30);
    }

    #[test]
    fn comparison_operators_count() {
        assert_eq!(COMPARISON_OPERATORS.len(), 7);
    }

    #[test]
    fn basic_operator_methods() {
        assert!(is_operator_method(b"+"));
        assert!(is_operator_method(b"=="));
        assert!(is_operator_method(b"[]"));
        assert!(is_operator_method(b"[]="));
        assert!(is_operator_method(b"!"));
        assert!(is_operator_method(b"+@"));
        assert!(is_operator_method(b"-@"));
        assert!(is_operator_method(b"`"));
        assert!(!is_operator_method(b"foo"));
        assert!(!is_operator_method(b"foo="));
    }

    #[test]
    fn setter_methods() {
        assert!(is_setter_method(b"foo="));
        assert!(is_setter_method(b"bar="));
        assert!(is_setter_method(b"[]="));
        // Comparison operators end with = but are NOT setters
        assert!(!is_setter_method(b"=="));
        assert!(!is_setter_method(b"!="));
        assert!(!is_setter_method(b"==="));
        assert!(!is_setter_method(b"<="));
        assert!(!is_setter_method(b">="));
        // Regular methods are not setters
        assert!(!is_setter_method(b"foo"));
        assert!(!is_setter_method(b"bar?"));
    }

    #[test]
    fn comparison_methods() {
        assert!(is_comparison_method(b"=="));
        assert!(is_comparison_method(b"==="));
        assert!(is_comparison_method(b"!="));
        assert!(is_comparison_method(b"<="));
        assert!(is_comparison_method(b">="));
        assert!(is_comparison_method(b">"));
        assert!(is_comparison_method(b"<"));
        assert!(!is_comparison_method(b"<=>"));
        assert!(!is_comparison_method(b"foo"));
    }

    #[test]
    fn assignment_methods() {
        assert!(is_assignment_method(b"foo="));
        assert!(!is_assignment_method(b"=="));
        assert!(!is_assignment_method(b"!="));
    }
}
