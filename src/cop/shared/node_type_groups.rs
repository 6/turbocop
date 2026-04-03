//! Node type group predicates, mirroring rubocop-ast's `Node::GROUP_FOR_TYPE`.
//!
//! Canonical source:
//! `vendor/rubocop-ast/lib/rubocop/ast/node.rb` (`GROUP_FOR_TYPE` constant)
//!
//! These predicates work with the `u8` tags from `node_type::node_type_tag()`,
//! enabling O(1) grouped type checks.
//!
//! ## Prism vs parser gem differences
//!
//! - Prism has a single `DefNode` (no separate `defs` for `def self.foo`).
//! - Prism has a single `CallNode` (no separate `send`/`csend`); safe navigation
//!   is a property of the call operator, not a distinct node type.
//! - Prism uses `LambdaNode` for stabby lambdas (`-> {}`), separate from `BlockNode`.
//! - Prism has `IfNode` and `UnlessNode` as separate types (parser uses `if` for both).

use super::node_type::*;

// ---------------------------------------------------------------------------
// Grouped type predicates
// ---------------------------------------------------------------------------

/// Check if a node type tag is a method definition (`def` or `defs` in parser gem terms).
///
/// Matches rubocop-ast's `any_def_type?` group: `%i[def defs]`.
///
/// In Prism, both `def foo` and `def self.foo` produce `DefNode`.
pub fn is_any_def_type(tag: u8) -> bool {
    tag == DEF_NODE
}

/// Check if a node type tag is any block type.
///
/// Matches rubocop-ast's `any_block_type?` group: `%i[block numblock itblock]`.
///
/// In Prism, `BlockNode` covers regular blocks, numblocks, and itblocks.
/// `LambdaNode` covers stabby lambdas (`-> {}`).
pub fn is_any_block_type(tag: u8) -> bool {
    matches!(tag, BLOCK_NODE | LAMBDA_NODE)
}

/// Check if a node type tag is any string type.
///
/// Matches rubocop-ast's `any_str_type?` group: `%i[str dstr xstr]`.
pub fn is_any_string_type(tag: u8) -> bool {
    matches!(
        tag,
        STRING_NODE | INTERPOLATED_STRING_NODE | X_STRING_NODE | INTERPOLATED_X_STRING_NODE
    )
}

/// Check if a node type tag is any symbol type.
///
/// Matches rubocop-ast's `any_sym_type?` group: `%i[sym dsym]`.
pub fn is_any_symbol_type(tag: u8) -> bool {
    matches!(tag, SYMBOL_NODE | INTERPOLATED_SYMBOL_NODE)
}

/// Check if a node type tag is a boolean literal.
///
/// Matches rubocop-ast's `boolean_type?` group: `%i[true false]`.
pub fn is_boolean_type(tag: u8) -> bool {
    matches!(tag, TRUE_NODE | FALSE_NODE)
}

/// Check if a node type tag is a numeric literal.
///
/// Matches rubocop-ast's `numeric_type?` group: `%i[int float rational complex]`.
pub fn is_numeric_type(tag: u8) -> bool {
    matches!(
        tag,
        INTEGER_NODE | FLOAT_NODE | RATIONAL_NODE | IMAGINARY_NODE
    )
}

/// Check if a node type tag is a range literal.
///
/// Matches rubocop-ast's `range_type?` group: `%i[irange erange]`.
///
/// In Prism, both inclusive (`..`) and exclusive (`...`) ranges use `RangeNode`.
pub fn is_range_type(tag: u8) -> bool {
    tag == RANGE_NODE
}

/// Check if a node type tag is a method call.
///
/// Matches rubocop-ast's `call_type?` group: `%i[send csend]`.
///
/// In Prism, both regular calls and safe-navigation calls use `CallNode`.
pub fn is_call_type(tag: u8) -> bool {
    tag == CALL_NODE
}

/// Check if a node type tag is a method/block parameter type.
///
/// Matches rubocop-ast's `argument_type?` group:
///   `%i[arg optarg restarg kwarg kwoptarg kwrestarg blockarg forward_arg shadowarg]`
pub fn is_argument_type(tag: u8) -> bool {
    matches!(
        tag,
        REQUIRED_PARAMETER_NODE
            | OPTIONAL_PARAMETER_NODE
            | REST_PARAMETER_NODE
            | KEYWORD_REST_PARAMETER_NODE
            | REQUIRED_KEYWORD_PARAMETER_NODE
            | OPTIONAL_KEYWORD_PARAMETER_NODE
            | BLOCK_PARAMETER_NODE
            | FORWARDING_PARAMETER_NODE
    )
}

/// Check if a node type tag is a variable read.
///
/// Matches rubocop-ast's `VARIABLES`: `%i[ivar gvar cvar lvar]`.
pub fn is_variable_type(tag: u8) -> bool {
    matches!(
        tag,
        LOCAL_VARIABLE_READ_NODE
            | INSTANCE_VARIABLE_READ_NODE
            | CLASS_VARIABLE_READ_NODE
            | GLOBAL_VARIABLE_READ_NODE
    )
}

/// Check if a node type tag is a variable/constant write (assignment).
///
/// Matches rubocop-ast's `ASSIGNMENTS`:
///   `%i[lvasgn ivasgn cvasgn gvasgn casgn masgn op_asgn or_asgn and_asgn]`
pub fn is_assignment_type(tag: u8) -> bool {
    matches!(
        tag,
        LOCAL_VARIABLE_WRITE_NODE
            | INSTANCE_VARIABLE_WRITE_NODE
            | CLASS_VARIABLE_WRITE_NODE
            | GLOBAL_VARIABLE_WRITE_NODE
            | CONSTANT_WRITE_NODE
            | CONSTANT_PATH_WRITE_NODE
            | MULTI_WRITE_NODE
            | LOCAL_VARIABLE_AND_WRITE_NODE
            | LOCAL_VARIABLE_OR_WRITE_NODE
            | LOCAL_VARIABLE_OPERATOR_WRITE_NODE
            | INSTANCE_VARIABLE_AND_WRITE_NODE
            | INSTANCE_VARIABLE_OR_WRITE_NODE
            | INSTANCE_VARIABLE_OPERATOR_WRITE_NODE
            | CLASS_VARIABLE_AND_WRITE_NODE
            | CLASS_VARIABLE_OR_WRITE_NODE
            | CLASS_VARIABLE_OPERATOR_WRITE_NODE
            | GLOBAL_VARIABLE_AND_WRITE_NODE
            | GLOBAL_VARIABLE_OR_WRITE_NODE
            | GLOBAL_VARIABLE_OPERATOR_WRITE_NODE
            | CONSTANT_AND_WRITE_NODE
            | CONSTANT_OR_WRITE_NODE
            | CONSTANT_OPERATOR_WRITE_NODE
            | CONSTANT_PATH_AND_WRITE_NODE
            | CONSTANT_PATH_OR_WRITE_NODE
            | CONSTANT_PATH_OPERATOR_WRITE_NODE
    )
}

/// Check if a node type tag is a conditional.
///
/// Matches rubocop-ast's `CONDITIONALS`: `%i[if while until case case_match]`.
///
/// In Prism, `if` and `unless` are separate node types.
pub fn is_conditional_type(tag: u8) -> bool {
    matches!(
        tag,
        IF_NODE | UNLESS_NODE | WHILE_NODE | UNTIL_NODE | CASE_NODE | CASE_MATCH_NODE
    )
}

/// Check if a node type tag is a basic conditional (excludes case).
///
/// Matches rubocop-ast's `BASIC_CONDITIONALS`: `%i[if while until]`.
pub fn is_basic_conditional_type(tag: u8) -> bool {
    matches!(tag, IF_NODE | UNLESS_NODE | WHILE_NODE | UNTIL_NODE)
}

/// Check if a node type tag is a loop construct.
///
/// Matches rubocop-ast's `LOOP_TYPES`: `%i[while until while_post until_post for]`.
///
/// In Prism, post-condition loops are represented by the same node types with
/// different flags, so this only checks `WhileNode | UntilNode | ForNode`.
pub fn is_loop_type(tag: u8) -> bool {
    matches!(tag, WHILE_NODE | UNTIL_NODE | FOR_NODE)
}

// ---------------------------------------------------------------------------
// Node-level convenience wrappers
// ---------------------------------------------------------------------------
// These accept a `&ruby_prism::Node` directly so callers don't need to import
// `node_type_tag` separately.

use super::node_type::node_type_tag;

/// Check if a `Node` is a boolean literal (`true` or `false`).
pub fn is_boolean_node(node: &ruby_prism::Node<'_>) -> bool {
    is_boolean_type(node_type_tag(node))
}

/// Check if a `Node` is a numeric literal (integer, float, rational, or imaginary).
pub fn is_numeric_node(node: &ruby_prism::Node<'_>) -> bool {
    is_numeric_type(node_type_tag(node))
}

/// Check if a `Node` is any string type (string, interpolated string, xstring).
pub fn is_any_string_node(node: &ruby_prism::Node<'_>) -> bool {
    is_any_string_type(node_type_tag(node))
}

/// Check if a `Node` is any symbol type (symbol or interpolated symbol).
pub fn is_any_symbol_node(node: &ruby_prism::Node<'_>) -> bool {
    is_any_symbol_type(node_type_tag(node))
}

/// Check if a `Node` is any block type (block or lambda).
pub fn is_any_block_node(node: &ruby_prism::Node<'_>) -> bool {
    is_any_block_type(node_type_tag(node))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_def_type() {
        assert!(is_any_def_type(DEF_NODE));
        assert!(!is_any_def_type(CALL_NODE));
        assert!(!is_any_def_type(BLOCK_NODE));
    }

    #[test]
    fn test_any_block_type() {
        assert!(is_any_block_type(BLOCK_NODE));
        assert!(is_any_block_type(LAMBDA_NODE));
        assert!(!is_any_block_type(DEF_NODE));
        assert!(!is_any_block_type(CALL_NODE));
    }

    #[test]
    fn test_any_string_type() {
        assert!(is_any_string_type(STRING_NODE));
        assert!(is_any_string_type(INTERPOLATED_STRING_NODE));
        assert!(is_any_string_type(X_STRING_NODE));
        assert!(is_any_string_type(INTERPOLATED_X_STRING_NODE));
        assert!(!is_any_string_type(SYMBOL_NODE));
        assert!(!is_any_string_type(INTEGER_NODE));
    }

    #[test]
    fn test_any_symbol_type() {
        assert!(is_any_symbol_type(SYMBOL_NODE));
        assert!(is_any_symbol_type(INTERPOLATED_SYMBOL_NODE));
        assert!(!is_any_symbol_type(STRING_NODE));
    }

    #[test]
    fn test_boolean_type() {
        assert!(is_boolean_type(TRUE_NODE));
        assert!(is_boolean_type(FALSE_NODE));
        assert!(!is_boolean_type(NIL_NODE));
        assert!(!is_boolean_type(INTEGER_NODE));
    }

    #[test]
    fn test_numeric_type() {
        assert!(is_numeric_type(INTEGER_NODE));
        assert!(is_numeric_type(FLOAT_NODE));
        assert!(is_numeric_type(RATIONAL_NODE));
        assert!(is_numeric_type(IMAGINARY_NODE));
        assert!(!is_numeric_type(STRING_NODE));
        assert!(!is_numeric_type(TRUE_NODE));
    }

    #[test]
    fn test_range_type() {
        assert!(is_range_type(RANGE_NODE));
        assert!(!is_range_type(INTEGER_NODE));
    }

    #[test]
    fn test_call_type() {
        assert!(is_call_type(CALL_NODE));
        assert!(!is_call_type(DEF_NODE));
        assert!(!is_call_type(BLOCK_NODE));
    }

    #[test]
    fn test_argument_type() {
        assert!(is_argument_type(REQUIRED_PARAMETER_NODE));
        assert!(is_argument_type(OPTIONAL_PARAMETER_NODE));
        assert!(is_argument_type(REST_PARAMETER_NODE));
        assert!(is_argument_type(KEYWORD_REST_PARAMETER_NODE));
        assert!(is_argument_type(BLOCK_PARAMETER_NODE));
        assert!(is_argument_type(FORWARDING_PARAMETER_NODE));
        assert!(!is_argument_type(CALL_NODE));
        assert!(!is_argument_type(DEF_NODE));
    }

    #[test]
    fn test_variable_type() {
        assert!(is_variable_type(LOCAL_VARIABLE_READ_NODE));
        assert!(is_variable_type(INSTANCE_VARIABLE_READ_NODE));
        assert!(is_variable_type(CLASS_VARIABLE_READ_NODE));
        assert!(is_variable_type(GLOBAL_VARIABLE_READ_NODE));
        assert!(!is_variable_type(LOCAL_VARIABLE_WRITE_NODE));
        assert!(!is_variable_type(CONSTANT_READ_NODE));
    }

    #[test]
    fn test_assignment_type() {
        assert!(is_assignment_type(LOCAL_VARIABLE_WRITE_NODE));
        assert!(is_assignment_type(INSTANCE_VARIABLE_WRITE_NODE));
        assert!(is_assignment_type(CLASS_VARIABLE_WRITE_NODE));
        assert!(is_assignment_type(GLOBAL_VARIABLE_WRITE_NODE));
        assert!(is_assignment_type(CONSTANT_WRITE_NODE));
        assert!(is_assignment_type(MULTI_WRITE_NODE));
        assert!(is_assignment_type(LOCAL_VARIABLE_OR_WRITE_NODE));
        assert!(is_assignment_type(CONSTANT_PATH_WRITE_NODE));
        assert!(!is_assignment_type(LOCAL_VARIABLE_READ_NODE));
        assert!(!is_assignment_type(CALL_NODE));
    }

    #[test]
    fn test_conditional_type() {
        assert!(is_conditional_type(IF_NODE));
        assert!(is_conditional_type(UNLESS_NODE));
        assert!(is_conditional_type(CASE_NODE));
        assert!(is_conditional_type(CASE_MATCH_NODE));
        assert!(is_conditional_type(WHILE_NODE));
        assert!(is_conditional_type(UNTIL_NODE));
        assert!(!is_conditional_type(DEF_NODE));
    }

    #[test]
    fn test_basic_conditional_type() {
        assert!(is_basic_conditional_type(IF_NODE));
        assert!(is_basic_conditional_type(UNLESS_NODE));
        assert!(is_basic_conditional_type(WHILE_NODE));
        assert!(is_basic_conditional_type(UNTIL_NODE));
        assert!(!is_basic_conditional_type(CASE_NODE));
        assert!(!is_basic_conditional_type(CASE_MATCH_NODE));
    }

    #[test]
    fn test_loop_type() {
        assert!(is_loop_type(WHILE_NODE));
        assert!(is_loop_type(UNTIL_NODE));
        assert!(is_loop_type(FOR_NODE));
        assert!(!is_loop_type(IF_NODE));
        assert!(!is_loop_type(BLOCK_NODE));
    }
}
