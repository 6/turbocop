//! Shared structural method dispatch predicates, mirroring rubocop-ast's
//! `MethodDispatchNode` mixin.
//!
//! Canonical source:
//! `vendor/rubocop-ast/lib/rubocop/ast/node/mixin/method_dispatch_node.rb`
//!
//! These predicates check the **shape** of a `CallNode` (receiver, operator,
//! block presence, etc.). Name-based predicates (operator?, setter?, bang?,
//! etc.) live in `method_identifier_predicates.rs`.

use crate::cop::shared::method_identifier_predicates::is_operator_method;

// ---------------------------------------------------------------------------
// Call operator predicates
// ---------------------------------------------------------------------------

/// Check if a call uses a dot (`.`) to connect receiver and method name.
///
/// Matches rubocop-ast's `MethodDispatchNode#dot?`.
///
/// ```ruby
/// foo.bar   # true
/// foo::bar  # false
/// foo&.bar  # false
/// foo + bar # false
/// ```
pub fn is_dot_call(call: &ruby_prism::CallNode<'_>) -> bool {
    call.call_operator_loc()
        .is_some_and(|loc| loc.as_slice() == b".")
}

/// Check if a call uses a double colon (`::`) to connect receiver and method.
///
/// Matches rubocop-ast's `MethodDispatchNode#double_colon?`.
///
/// ```ruby
/// Foo::bar  # true
/// foo.bar   # false
/// ```
pub fn is_double_colon_call(call: &ruby_prism::CallNode<'_>) -> bool {
    call.call_operator_loc()
        .is_some_and(|loc| loc.as_slice() == b"::")
}

/// Check if a call uses safe navigation (`&.`).
///
/// Matches rubocop-ast's `MethodDispatchNode#safe_navigation?`.
///
/// ```ruby
/// foo&.bar  # true
/// foo.bar   # false
/// ```
pub fn is_safe_navigation(call: &ruby_prism::CallNode<'_>) -> bool {
    call.call_operator_loc()
        .is_some_and(|loc| loc.as_slice() == b"&.")
}

// ---------------------------------------------------------------------------
// Receiver predicates
// ---------------------------------------------------------------------------

/// Check if a call has no receiver and matches a given method name.
///
/// Matches rubocop-ast's `MethodDispatchNode#command?(name)`:
///   `!receiver && method?(name)`
///
/// ```ruby
/// puts "hi"    # is_command(call, b"puts") → true
/// self.puts    # is_command(call, b"puts") → false (has receiver)
/// ```
pub fn is_command(call: &ruby_prism::CallNode<'_>, name: &[u8]) -> bool {
    call.receiver().is_none() && call.name().as_slice() == name
}

/// Check if the explicit receiver of a call is `self`.
///
/// Matches rubocop-ast's `MethodDispatchNode#self_receiver?`.
///
/// ```ruby
/// self.foo  # true
/// foo       # false (no receiver at all)
/// bar.foo   # false
/// ```
pub fn is_self_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    call.receiver().is_some_and(|r| r.as_self_node().is_some())
}

/// Check if the explicit receiver of a call is a constant.
///
/// Matches rubocop-ast's `MethodDispatchNode#const_receiver?`.
///
/// ```ruby
/// Foo.bar       # true  (ConstantReadNode)
/// Foo::Bar.baz  # true  (ConstantPathNode)
/// foo.bar       # false
/// ```
pub fn is_const_receiver(call: &ruby_prism::CallNode<'_>) -> bool {
    call.receiver()
        .is_some_and(|r| r.as_constant_read_node().is_some() || r.as_constant_path_node().is_some())
}

// ---------------------------------------------------------------------------
// Block & modifier predicates
// ---------------------------------------------------------------------------

/// Check if a call has an associated block (do...end or {...}).
///
/// Matches rubocop-ast's `MethodDispatchNode#block_literal?`.
///
/// ```ruby
/// foo { }      # true
/// foo do end   # true
/// foo          # false
/// ```
pub fn has_block(call: &ruby_prism::CallNode<'_>) -> bool {
    call.block().is_some()
}

/// Check if a call is a setter method dispatch (has operator loc for `=`).
///
/// Matches rubocop-ast's `MethodDispatchNode#setter_method?`.
///
/// ```ruby
/// foo.bar = 1  # true  (has operator_loc for =)
/// foo.bar      # false
/// ```
pub fn is_setter_call(call: &ruby_prism::CallNode<'_>) -> bool {
    // Delegate to the name-based `is_setter_method` which already handles
    // the "ends with = but not a comparison operator" logic correctly.
    use crate::cop::shared::method_identifier_predicates::is_setter_method;
    is_setter_method(call.name().as_slice())
}

/// Check if a call is the implicit form of `#call`, e.g. `foo.(bar)`.
///
/// Matches rubocop-ast's `MethodDispatchNode#implicit_call?`.
///
/// ```ruby
/// foo.(bar)  # true  (name is "call" but no message_loc/selector)
/// foo.call   # false (has explicit "call" selector)
/// ```
pub fn is_implicit_call(call: &ruby_prism::CallNode<'_>) -> bool {
    call.name().as_slice() == b"call" && call.message_loc().is_none()
}

// ---------------------------------------------------------------------------
// Operator form predicates
// ---------------------------------------------------------------------------

/// Check if a call is a unary operation (e.g. `-foo`, `!bar`, `~baz`).
///
/// Matches rubocop-ast's `MethodDispatchNode#unary_operation?`:
/// An operator method where the expression starts at the selector position
/// (no receiver before the operator in source).
///
/// ```ruby
/// -foo   # true  (unary minus)
/// !bar   # true  (unary not)
/// a + b  # false (binary)
/// ```
pub fn is_unary_operation(call: &ruby_prism::CallNode<'_>) -> bool {
    let Some(message_loc) = call.message_loc() else {
        return false;
    };
    is_operator_method(call.name().as_slice())
        && call.location().start_offset() == message_loc.start_offset()
}

/// Check if a call is a binary operation (e.g. `a + b`, `x == y`).
///
/// Matches rubocop-ast's `MethodDispatchNode#binary_operation?`:
/// An operator method where the expression starts before the selector
/// (receiver precedes the operator in source).
///
/// ```ruby
/// a + b   # true
/// x == y  # true
/// -foo    # false (unary)
/// ```
pub fn is_binary_operation(call: &ruby_prism::CallNode<'_>) -> bool {
    let Some(message_loc) = call.message_loc() else {
        return false;
    };
    is_operator_method(call.name().as_slice())
        && call.location().start_offset() != message_loc.start_offset()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_source;

    /// Parse Ruby source and extract the first CallNode found at the top level.
    fn parse_call(code: &str) -> bool {
        // Sanity: just ensure parsing works. Actual tests use specific helpers below.
        let result = parse_source(code.as_bytes());
        let program = result.node();
        program.as_program_node().is_some()
    }

    /// Helper: parse code, get the first statement, try to cast to CallNode, apply predicate.
    fn check_call<F: Fn(&ruby_prism::CallNode<'_>) -> bool>(code: &str, f: F) -> bool {
        let result = parse_source(code.as_bytes());
        let program = result.node();
        let program = program.as_program_node().unwrap();
        let stmts = program.statements();
        let node = stmts.body().iter().next().unwrap();
        if let Some(call) = node.as_call_node() {
            f(&call)
        } else {
            panic!("Expected CallNode, got: {code}");
        }
    }

    #[test]
    fn test_parse_sanity() {
        assert!(parse_call("foo.bar"));
    }

    // --- Call operator predicates ---

    #[test]
    fn test_is_dot_call() {
        assert!(check_call("foo.bar", is_dot_call));
        assert!(!check_call("foo::bar", is_dot_call));
        assert!(!check_call("foo&.bar", is_dot_call));
        assert!(!check_call("bar()", is_dot_call)); // no call_operator
    }

    #[test]
    fn test_is_double_colon_call() {
        assert!(check_call("foo::bar", is_double_colon_call));
        assert!(!check_call("foo.bar", is_double_colon_call));
        assert!(!check_call("foo&.bar", is_double_colon_call));
    }

    #[test]
    fn test_is_safe_navigation() {
        assert!(check_call("foo&.bar", is_safe_navigation));
        assert!(!check_call("foo.bar", is_safe_navigation));
        assert!(!check_call("foo::bar", is_safe_navigation));
    }

    // --- Receiver predicates ---

    #[test]
    fn test_is_command() {
        assert!(check_call("puts 'hi'", |c| is_command(c, b"puts")));
        assert!(!check_call("puts 'hi'", |c| is_command(c, b"print")));
        assert!(!check_call("self.puts", |c| is_command(c, b"puts")));
        assert!(!check_call("foo.puts", |c| is_command(c, b"puts")));
    }

    #[test]
    fn test_is_self_receiver() {
        assert!(check_call("self.foo", is_self_receiver));
        assert!(!check_call("bar.foo", is_self_receiver));
        assert!(!check_call("foo", is_self_receiver));
    }

    #[test]
    fn test_is_const_receiver() {
        assert!(check_call("Foo.bar", is_const_receiver));
        assert!(check_call("Foo::Bar.baz", is_const_receiver));
        assert!(!check_call("foo.bar", is_const_receiver));
        assert!(!check_call("self.bar", is_const_receiver));
        assert!(!check_call("bar", is_const_receiver));
    }

    // --- Block & modifier predicates ---

    #[test]
    fn test_has_block() {
        assert!(check_call("foo { }", has_block));
        assert!(!check_call("foo", has_block));
        assert!(!check_call("foo(1)", has_block));
    }

    #[test]
    fn test_is_setter_call() {
        assert!(check_call("foo.bar = 1", is_setter_call));
        assert!(!check_call("foo.bar", is_setter_call));
        // Operators like == are NOT setters
        assert!(!check_call("foo == bar", is_setter_call));
    }

    #[test]
    fn test_is_implicit_call() {
        assert!(check_call("foo.(1)", is_implicit_call));
        assert!(!check_call("foo.call(1)", is_implicit_call));
        assert!(!check_call("foo.bar", is_implicit_call));
    }

    // --- Operator form predicates ---

    #[test]
    fn test_is_unary_operation() {
        assert!(check_call("-foo", is_unary_operation));
        assert!(check_call("!bar", is_unary_operation));
        assert!(check_call("~baz", is_unary_operation));
        assert!(!check_call("a + b", is_unary_operation));
        assert!(!check_call("foo.bar", is_unary_operation));
    }

    #[test]
    fn test_is_binary_operation() {
        assert!(check_call("a + b", is_binary_operation));
        assert!(check_call("x == y", is_binary_operation));
        assert!(check_call("a <=> b", is_binary_operation));
        assert!(!check_call("-foo", is_binary_operation));
        assert!(!check_call("foo.bar", is_binary_operation));
    }
}
