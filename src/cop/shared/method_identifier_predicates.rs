//! Shared method identifier predicates, mirroring rubocop-ast's
//! `MethodIdentifierPredicates` module.
//!
//! Canonical source:
//! `vendor/rubocop-ast/lib/rubocop/ast/node/mixin/method_identifier_predicates.rb`
//! `vendor/rubocop-ast/lib/rubocop/ast/node/mixin/method_dispatch_node.rb`

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

/// Enumerator methods from rubocop-ast `ENUMERATOR_METHODS`.
///
/// Note: `is_enumerator_method()` also matches any method starting with `each_`.
pub const ENUMERATOR_METHODS: &[&[u8]] = &[
    b"collect",
    b"collect_concat",
    b"detect",
    b"downto",
    b"each",
    b"find",
    b"find_all",
    b"find_index",
    b"inject",
    b"loop",
    b"map",
    b"map!",
    b"reduce",
    b"reject",
    b"reject!",
    b"reverse_each",
    b"select",
    b"select!",
    b"times",
    b"upto",
];

/// Ruby's `Enumerable` instance methods plus `:each`.
///
/// From rubocop-ast: `(Enumerable.instance_methods + [:each]).to_set.freeze`
pub const ENUMERABLE_METHODS: &[&[u8]] = &[
    b"all?",
    b"any?",
    b"chain",
    b"chunk",
    b"chunk_while",
    b"collect",
    b"collect_concat",
    b"compact",
    b"count",
    b"cycle",
    b"detect",
    b"drop",
    b"drop_while",
    b"each",
    b"each_cons",
    b"each_entry",
    b"each_slice",
    b"each_with_index",
    b"each_with_object",
    b"entries",
    b"filter",
    b"filter_map",
    b"find",
    b"find_all",
    b"find_index",
    b"first",
    b"flat_map",
    b"grep",
    b"grep_v",
    b"group_by",
    b"include?",
    b"inject",
    b"lazy",
    b"map",
    b"max",
    b"max_by",
    b"member?",
    b"min",
    b"min_by",
    b"minmax",
    b"minmax_by",
    b"none?",
    b"one?",
    b"partition",
    b"reduce",
    b"reject",
    b"reverse_each",
    b"select",
    b"slice_after",
    b"slice_before",
    b"slice_when",
    b"sort",
    b"sort_by",
    b"sum",
    b"take",
    b"take_while",
    b"tally",
    b"to_a",
    b"to_h",
    b"to_set",
    b"uniq",
    b"zip",
];

/// Non-mutating binary operator methods.
///
/// From rubocop-ast `NONMUTATING_BINARY_OPERATOR_METHODS`.
pub const NONMUTATING_BINARY_OPERATOR_METHODS: &[&[u8]] = &[
    b"*", b"/", b"%", b"+", b"-", b"==", b"===", b"!=", b"<", b">", b"<=", b">=", b"<=>",
];

/// Non-mutating unary operator methods.
///
/// From rubocop-ast `NONMUTATING_UNARY_OPERATOR_METHODS`.
pub const NONMUTATING_UNARY_OPERATOR_METHODS: &[&[u8]] = &[b"+@", b"-@", b"~", b"!"];

/// Non-mutating Array methods.
///
/// From rubocop-ast `NONMUTATING_ARRAY_METHODS`.
pub const NONMUTATING_ARRAY_METHODS: &[&[u8]] = &[
    b"all?",
    b"any?",
    b"assoc",
    b"at",
    b"bsearch",
    b"bsearch_index",
    b"collect",
    b"combination",
    b"compact",
    b"count",
    b"cycle",
    b"deconstruct",
    b"difference",
    b"dig",
    b"drop",
    b"drop_while",
    b"each",
    b"each_index",
    b"empty?",
    b"eql?",
    b"fetch",
    b"filter",
    b"find_index",
    b"first",
    b"flatten",
    b"hash",
    b"include?",
    b"index",
    b"inspect",
    b"intersection",
    b"join",
    b"last",
    b"length",
    b"map",
    b"max",
    b"min",
    b"minmax",
    b"none?",
    b"one?",
    b"pack",
    b"permutation",
    b"product",
    b"rassoc",
    b"reject",
    b"repeated_combination",
    b"repeated_permutation",
    b"reverse",
    b"reverse_each",
    b"rindex",
    b"rotate",
    b"sample",
    b"select",
    b"shuffle",
    b"size",
    b"slice",
    b"sort",
    b"sum",
    b"take",
    b"take_while",
    b"to_a",
    b"to_ary",
    b"to_h",
    b"to_s",
    b"transpose",
    b"union",
    b"uniq",
    b"values_at",
    b"zip",
    b"|",
];

/// Non-mutating Hash methods.
///
/// From rubocop-ast `NONMUTATING_HASH_METHODS`.
pub const NONMUTATING_HASH_METHODS: &[&[u8]] = &[
    b"any?",
    b"assoc",
    b"compact",
    b"dig",
    b"each",
    b"each_key",
    b"each_pair",
    b"each_value",
    b"empty?",
    b"eql?",
    b"fetch",
    b"fetch_values",
    b"filter",
    b"flatten",
    b"has_key?",
    b"has_value?",
    b"hash",
    b"include?",
    b"inspect",
    b"invert",
    b"key",
    b"key?",
    b"keys?",
    b"length",
    b"member?",
    b"merge",
    b"rassoc",
    b"rehash",
    b"reject",
    b"select",
    b"size",
    b"slice",
    b"to_a",
    b"to_h",
    b"to_hash",
    b"to_proc",
    b"to_s",
    b"transform_keys",
    b"transform_values",
    b"value?",
    b"values",
    b"values_at",
];

/// Non-mutating String methods.
///
/// From rubocop-ast `NONMUTATING_STRING_METHODS`.
pub const NONMUTATING_STRING_METHODS: &[&[u8]] = &[
    b"ascii_only?",
    b"b",
    b"bytes",
    b"bytesize",
    b"byteslice",
    b"capitalize",
    b"casecmp",
    b"casecmp?",
    b"center",
    b"chars",
    b"chomp",
    b"chop",
    b"chr",
    b"codepoints",
    b"count",
    b"crypt",
    b"delete",
    b"delete_prefix",
    b"delete_suffix",
    b"downcase",
    b"dump",
    b"each_byte",
    b"each_char",
    b"each_codepoint",
    b"each_grapheme_cluster",
    b"each_line",
    b"empty?",
    b"encode",
    b"encoding",
    b"end_with?",
    b"eql?",
    b"getbyte",
    b"grapheme_clusters",
    b"gsub",
    b"hash",
    b"hex",
    b"include",
    b"index",
    b"inspect",
    b"intern",
    b"length",
    b"lines",
    b"ljust",
    b"lstrip",
    b"match",
    b"match?",
    b"next",
    b"oct",
    b"ord",
    b"partition",
    b"reverse",
    b"rindex",
    b"rjust",
    b"rpartition",
    b"rstrip",
    b"scan",
    b"scrub",
    b"size",
    b"slice",
    b"squeeze",
    b"start_with?",
    b"strip",
    b"sub",
    b"succ",
    b"sum",
    b"swapcase",
    b"to_a",
    b"to_c",
    b"to_f",
    b"to_i",
    b"to_r",
    b"to_s",
    b"to_str",
    b"to_sym",
    b"tr",
    b"tr_s",
    b"unicode_normalize",
    b"unicode_normalized?",
    b"unpack",
    b"unpack1",
    b"upcase",
    b"upto",
    b"valid_encoding?",
];

/// Arithmetic operator methods.
///
/// From rubocop-ast `MethodDispatchNode::ARITHMETIC_OPERATORS`.
pub const ARITHMETIC_OPERATORS: &[&[u8]] = &[b"+", b"-", b"*", b"/", b"%", b"**"];

// ---------------------------------------------------------------------------
// Predicate functions
// ---------------------------------------------------------------------------

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

/// Check if a method name is an enumerator method.
///
/// Matches rubocop-ast's `enumerator_method?`:
///   `ENUMERATOR_METHODS.include?(method_name) || method_name.to_s.start_with?('each_')`
pub fn is_enumerator_method(name: &[u8]) -> bool {
    ENUMERATOR_METHODS.contains(&name) || name.starts_with(b"each_")
}

/// Check if a method name is an Enumerable method.
///
/// Matches rubocop-ast's `enumerable_method?`.
pub fn is_enumerable_method(name: &[u8]) -> bool {
    ENUMERABLE_METHODS.contains(&name)
}

/// Check if a method name is a non-mutating binary operator method.
pub fn is_nonmutating_binary_operator_method(name: &[u8]) -> bool {
    NONMUTATING_BINARY_OPERATOR_METHODS.contains(&name)
}

/// Check if a method name is a non-mutating unary operator method.
pub fn is_nonmutating_unary_operator_method(name: &[u8]) -> bool {
    NONMUTATING_UNARY_OPERATOR_METHODS.contains(&name)
}

/// Check if a method name is a non-mutating operator method (binary or unary).
pub fn is_nonmutating_operator_method(name: &[u8]) -> bool {
    is_nonmutating_binary_operator_method(name) || is_nonmutating_unary_operator_method(name)
}

/// Check if a method name is a non-mutating Array method.
pub fn is_nonmutating_array_method(name: &[u8]) -> bool {
    NONMUTATING_ARRAY_METHODS.contains(&name)
}

/// Check if a method name is a non-mutating Hash method.
pub fn is_nonmutating_hash_method(name: &[u8]) -> bool {
    NONMUTATING_HASH_METHODS.contains(&name)
}

/// Check if a method name is a non-mutating String method.
pub fn is_nonmutating_string_method(name: &[u8]) -> bool {
    NONMUTATING_STRING_METHODS.contains(&name)
}

/// Check if a method name is a predicate method (ends with `?`).
///
/// Matches rubocop-ast's `predicate_method?`.
pub fn is_predicate_method(name: &[u8]) -> bool {
    name.ends_with(b"?")
}

/// Check if a method name is a bang method (ends with `!`).
///
/// Matches rubocop-ast's `bang_method?`.
pub fn is_bang_method(name: &[u8]) -> bool {
    name.ends_with(b"!")
}

/// Check if a method name is a camel-case method (starts with uppercase A-Z).
///
/// Matches rubocop-ast's `camel_case_method?`: `method_name.to_s =~ /\A[A-Z]/`
pub fn is_camel_case_method(name: &[u8]) -> bool {
    name.first().is_some_and(|&b| b.is_ascii_uppercase())
}

/// Check if a method name is a negation method (`!`).
///
/// Matches rubocop-ast's `negation_method?` (name-only check; the full check
/// also requires a receiver, but that's caller's responsibility).
pub fn is_negation_method(name: &[u8]) -> bool {
    name == b"!"
}

/// Check if a method name is an arithmetic operation.
///
/// Matches rubocop-ast's `MethodDispatchNode#arithmetic_operation?`.
pub fn is_arithmetic_operation(name: &[u8]) -> bool {
    ARITHMETIC_OPERATORS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_methods_count() {
        // | ^ & <=> == === =~ > >= < <= << >> + - * / % ** ~ +@ -@ !@ ~@ [] []= ! != !~ ` = 30
        assert_eq!(OPERATOR_METHODS.len(), 30);
    }

    #[test]
    fn comparison_operators_count() {
        assert_eq!(COMPARISON_OPERATORS.len(), 7);
    }

    #[test]
    fn enumerator_methods_count() {
        assert_eq!(ENUMERATOR_METHODS.len(), 20);
    }

    #[test]
    fn enumerable_methods_count() {
        assert_eq!(ENUMERABLE_METHODS.len(), 62);
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

    #[test]
    fn enumerator_methods() {
        assert!(is_enumerator_method(b"each"));
        assert!(is_enumerator_method(b"map"));
        assert!(is_enumerator_method(b"collect"));
        assert!(is_enumerator_method(b"select"));
        assert!(is_enumerator_method(b"inject"));
        assert!(is_enumerator_method(b"times"));
        // each_ prefix
        assert!(is_enumerator_method(b"each_with_index"));
        assert!(is_enumerator_method(b"each_slice"));
        assert!(!is_enumerator_method(b"foo"));
        assert!(!is_enumerator_method(b"puts"));
    }

    #[test]
    fn enumerable_methods() {
        assert!(is_enumerable_method(b"each"));
        assert!(is_enumerable_method(b"map"));
        assert!(is_enumerable_method(b"select"));
        assert!(is_enumerable_method(b"sort_by"));
        assert!(is_enumerable_method(b"zip"));
        assert!(is_enumerable_method(b"all?"));
        assert!(is_enumerable_method(b"any?"));
        assert!(!is_enumerable_method(b"foo"));
        assert!(!is_enumerable_method(b"puts"));
    }

    #[test]
    fn nonmutating_methods() {
        // Binary operators
        assert!(is_nonmutating_binary_operator_method(b"+"));
        assert!(is_nonmutating_binary_operator_method(b"<=>"));
        assert!(!is_nonmutating_binary_operator_method(b"<<"));

        // Unary operators
        assert!(is_nonmutating_unary_operator_method(b"~"));
        assert!(is_nonmutating_unary_operator_method(b"!"));
        assert!(!is_nonmutating_unary_operator_method(b"+"));

        // Combined
        assert!(is_nonmutating_operator_method(b"+"));
        assert!(is_nonmutating_operator_method(b"~"));

        // Array
        assert!(is_nonmutating_array_method(b"first"));
        assert!(is_nonmutating_array_method(b"length"));
        assert!(is_nonmutating_array_method(b"sort"));
        assert!(!is_nonmutating_array_method(b"push"));

        // Hash
        assert!(is_nonmutating_hash_method(b"keys?"));
        assert!(is_nonmutating_hash_method(b"values"));
        assert!(is_nonmutating_hash_method(b"merge"));
        assert!(!is_nonmutating_hash_method(b"delete"));

        // String
        assert!(is_nonmutating_string_method(b"length"));
        assert!(is_nonmutating_string_method(b"upcase"));
        assert!(is_nonmutating_string_method(b"strip"));
        assert!(!is_nonmutating_string_method(b"replace"));
    }

    #[test]
    fn predicate_and_bang_methods() {
        assert!(is_predicate_method(b"empty?"));
        assert!(is_predicate_method(b"nil?"));
        assert!(!is_predicate_method(b"foo"));
        assert!(!is_predicate_method(b"foo!"));

        assert!(is_bang_method(b"save!"));
        assert!(is_bang_method(b"sort!"));
        assert!(!is_bang_method(b"foo"));
        assert!(!is_bang_method(b"foo?"));
    }

    #[test]
    fn camel_case_methods() {
        assert!(is_camel_case_method(b"Integer"));
        assert!(is_camel_case_method(b"Float"));
        assert!(!is_camel_case_method(b"integer"));
        assert!(!is_camel_case_method(b""));
    }

    #[test]
    fn negation_methods() {
        assert!(is_negation_method(b"!"));
        assert!(!is_negation_method(b"not"));
        assert!(!is_negation_method(b"!="));
    }

    #[test]
    fn arithmetic_operations() {
        assert!(is_arithmetic_operation(b"+"));
        assert!(is_arithmetic_operation(b"-"));
        assert!(is_arithmetic_operation(b"*"));
        assert!(is_arithmetic_operation(b"/"));
        assert!(is_arithmetic_operation(b"%"));
        assert!(is_arithmetic_operation(b"**"));
        assert!(!is_arithmetic_operation(b"<<"));
        assert!(!is_arithmetic_operation(b"=="));
    }
}
