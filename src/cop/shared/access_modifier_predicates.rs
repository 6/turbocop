//! Shared access modifier predicates, mirroring rubocop-ast's
//! `MethodDispatchNode` access modifier methods.
//!
//! Canonical source:
//! `vendor/rubocop-ast/lib/rubocop/ast/node/mixin/method_dispatch_node.rb`
//!
//! ## Usage
//!
//! For simple name checks (no scope validation needed):
//! ```ignore
//! if is_access_modifier_name(call.name().as_slice()) { ... }
//! if is_bare_access_modifier(&call) { ... }
//! ```
//!
//! For scope-aware checks, maintain a `Vec<MacroScope>` in your visitor and
//! call the `push_*`/`pop`/`current_macro_scope` helpers in your `visit_*` methods.

/// The four canonical access modifier method names.
pub const ACCESS_MODIFIER_NAMES: &[&[u8]] =
    &[b"private", b"protected", b"public", b"module_function"];

// ---------------------------------------------------------------------------
// Standalone predicate functions (no scope context needed)
// ---------------------------------------------------------------------------

/// Check if a method name is one of the four access modifier names.
///
/// Matches: `private`, `protected`, `public`, `module_function`.
pub fn is_access_modifier_name(name: &[u8]) -> bool {
    matches!(
        name,
        b"private" | b"protected" | b"public" | b"module_function"
    )
}

/// Check if a CallNode is a bare access modifier declaration (no receiver, no args).
///
/// Matches rubocop-ast's `bare_access_modifier_declaration?`:
///   `(send nil? {:public :protected :private :module_function})`
///
/// Note: This does NOT check `in_macro_scope?`. Use `is_bare_access_modifier_in_scope`
/// for the full `bare_access_modifier?` check.
pub fn is_bare_access_modifier(call: &ruby_prism::CallNode<'_>) -> bool {
    call.receiver().is_none()
        && call.arguments().is_none()
        && call.block().is_none()
        && is_access_modifier_name(call.name().as_slice())
}

/// Check if a CallNode is a non-bare access modifier declaration (no receiver, with args).
///
/// Matches rubocop-ast's `non_bare_access_modifier_declaration?`:
///   `(send nil? {:public :protected :private :module_function} _+)`
pub fn is_non_bare_access_modifier(call: &ruby_prism::CallNode<'_>) -> bool {
    call.receiver().is_none()
        && call.arguments().is_some()
        && is_access_modifier_name(call.name().as_slice())
}

/// Check if a CallNode is any access modifier declaration (bare or non-bare, no receiver).
///
/// Matches rubocop-ast's `access_modifier?` (without `macro?` scope check).
pub fn is_access_modifier_declaration(call: &ruby_prism::CallNode<'_>) -> bool {
    call.receiver().is_none() && is_access_modifier_name(call.name().as_slice())
}

/// Check if a bare access modifier is a "special" modifier (only `private` or `protected`).
///
/// Matches rubocop-ast's `special_modifier?`:
///   `bare_access_modifier? && SPECIAL_MODIFIERS.include?(source)`
///
/// Note: Excludes `public` and `module_function`.
pub fn is_special_modifier_name(name: &[u8]) -> bool {
    matches!(name, b"private" | b"protected")
}

// ---------------------------------------------------------------------------
// Macro scope tracking (for cops that need in_macro_scope? during visitation)
// ---------------------------------------------------------------------------

/// Whether the current visitor position is in a macro scope.
///
/// Mirrors rubocop-ast's `in_macro_scope?` recursive parent-chain check:
/// - root → InMacroScope
/// - parent is class/module/sclass/class_constructor? → InMacroScope
/// - parent is kwbegin/begin/any_block/if(body, not condition) AND parent is in macro scope → InMacroScope
/// - parent is def/defs → NotMacroScope
/// - everything else → NotMacroScope
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MacroScope {
    InMacroScope,
    NotMacroScope,
}

impl MacroScope {
    pub fn is_macro(self) -> bool {
        self == MacroScope::InMacroScope
    }
}

/// Push a class/module/sclass scope (always enters macro scope).
pub fn push_class_like_scope(stack: &mut Vec<MacroScope>) {
    stack.push(MacroScope::InMacroScope);
}

/// Push a def/defs scope (always exits macro scope).
pub fn push_def_scope(stack: &mut Vec<MacroScope>) {
    stack.push(MacroScope::NotMacroScope);
}

/// Push a "wrapper" scope (begin, block, if-body, kwbegin).
///
/// Inherits the parent's macro scope: if the parent is in macro scope,
/// the wrapper is too. Otherwise, it's not.
///
/// This mirrors rubocop-ast's `in_macro_scope?` pattern:
/// ```text
/// [ { kwbegin begin any_block (if _condition <%0 _>) }
///   #in_macro_scope? ]
/// ```
pub fn push_wrapper_scope(stack: &mut Vec<MacroScope>) {
    let current = current_macro_scope(stack);
    stack.push(current);
}

/// Pop the most recent scope from the stack.
pub fn pop_scope(stack: &mut Vec<MacroScope>) {
    stack.pop();
}

/// Get the current macro scope. Returns `InMacroScope` if the stack is empty
/// (root level is macro scope).
pub fn current_macro_scope(stack: &[MacroScope]) -> MacroScope {
    stack.last().copied().unwrap_or(MacroScope::InMacroScope)
}

/// Check if currently in macro scope.
pub fn in_macro_scope(stack: &[MacroScope]) -> bool {
    current_macro_scope(stack).is_macro()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_modifier_names() {
        assert!(is_access_modifier_name(b"private"));
        assert!(is_access_modifier_name(b"protected"));
        assert!(is_access_modifier_name(b"public"));
        assert!(is_access_modifier_name(b"module_function"));
        assert!(!is_access_modifier_name(b"attr_reader"));
        assert!(!is_access_modifier_name(b"foo"));
    }

    #[test]
    fn special_modifier_names() {
        assert!(is_special_modifier_name(b"private"));
        assert!(is_special_modifier_name(b"protected"));
        assert!(!is_special_modifier_name(b"public"));
        assert!(!is_special_modifier_name(b"module_function"));
    }

    #[test]
    fn macro_scope_root() {
        let stack: Vec<MacroScope> = vec![];
        assert!(in_macro_scope(&stack));
    }

    #[test]
    fn macro_scope_class() {
        let mut stack = vec![];
        push_class_like_scope(&mut stack);
        assert!(in_macro_scope(&stack));
    }

    #[test]
    fn macro_scope_def_breaks_it() {
        let mut stack = vec![];
        push_class_like_scope(&mut stack);
        push_def_scope(&mut stack);
        assert!(!in_macro_scope(&stack));
    }

    #[test]
    fn macro_scope_wrapper_inherits() {
        let mut stack = vec![];
        push_class_like_scope(&mut stack);
        push_wrapper_scope(&mut stack); // begin/block in class body
        assert!(in_macro_scope(&stack));

        push_def_scope(&mut stack);
        push_wrapper_scope(&mut stack); // begin/block in def body
        assert!(!in_macro_scope(&stack));
    }

    #[test]
    fn macro_scope_pop_restores() {
        let mut stack = vec![];
        push_class_like_scope(&mut stack);
        push_def_scope(&mut stack);
        assert!(!in_macro_scope(&stack));
        pop_scope(&mut stack);
        assert!(in_macro_scope(&stack));
    }
}
