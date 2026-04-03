use ruby_prism::Visit;

use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Checks for nested method definitions.
///
/// ## Investigation findings
///
/// Root causes of FP/FN:
/// 1. **FN (def self.y):** nitrocop skipped ALL defs with receivers, but RuboCop only
///    skips defs whose receiver is a variable (local/ivar/cvar/gvar), constant, or
///    method call — NOT `self`. `def self.y` inside another def IS an offense.
/// 2. **FP (AllowedMethods/AllowedPatterns):** nitrocop checked these against the
///    *outer def's* name, but RuboCop checks them against *enclosing block call* names.
///    e.g., `AllowedMethods: ['has_many']` exempts `def` inside `has_many do...end` blocks.
/// 3. **FN (Data.define):** Missing `Data.define` as a scope-creating call (added in
///    Ruby 3.2, recognized by rubocop-ast's `class_constructor?`).
/// 4. **FP (scope-creating ancestor above outer def):** The old approach walked only
///    inside the outer def's body to find scope-creating blocks. But RuboCop checks
///    ALL ancestors of the inner def for scope-creating blocks — including those ABOVE
///    the outer def. For example, `def` nested inside `def` inside `Struct.new do...end`
///    or `class << self` was incorrectly flagged because the scope-creating ancestor
///    was above the outer def. Fix: switch from `check_node` on `DEF_NODE` to
///    `check_source` with a full-tree visitor that tracks both def depth and scope
///    depth across the entire AST.
/// 5. **FP (parenthesized allowed receivers / `it`, 2026-03-28):**
///    Prism preserves receiver parens as `ParenthesesNode`, so `has_allowed_receiver`
///    missed allowed receivers wrapped in parens and implicit `it` locals. RuboCop
///    still allows `def (ActiveRecord::Base.connection).index_name_exists?`,
///    `def (Kernel.const_get(...)).const_get`, `def (do_something&.y).z`, and
///    `def it.attached? = true` because the wrapped subject is still a call/const/variable.
///    Fix: unwrap single-expression parens and treat `ItLocalVariableReadNode` as allowed.
/// 6. **FN (qualified constructor paths, 2026-03-28):** `is_scope_creating_call` used
///    `constant_name()`, which only returns the last constant segment. That incorrectly
///    treated `Object::Module.new` as `Module.new`, suppressing real offenses like
///    nested `def self.session` in the SugarCRM corpus. Fix: require a simple/top-level
///    constant via `util::is_simple_constant()`, so only `Module.new`, `::Module.new`,
///    `Class.new`, `Struct.new`, and `Data.define` create scope.
pub struct NestedMethodDefinition;

/// Full-tree visitor that tracks def nesting depth and scope-creating context depth.
///
/// RuboCop's algorithm (on_def): for each inner def, check if there's a def ancestor
/// AND whether any ancestor block/sclass is scope-creating. If any scope-creating
/// ancestor exists anywhere above the inner def, the offense is suppressed.
///
/// This visitor mirrors that by maintaining:
/// - `def_depth`: incremented when entering any DefNode
/// - `scope_depth`: incremented when entering scope-creating blocks (class_eval,
///   Class.new, Struct.new, etc.) or singleton class nodes
///
/// A def is flagged only when `def_depth > 0` (inside another def) AND
/// `scope_depth == 0` (no scope-creating ancestor anywhere above).
struct FullTreeWalker<'a> {
    source: &'a SourceFile,
    cop: &'a NestedMethodDefinition,
    def_depth: usize,
    scope_depth: usize,
    allowed_methods: Option<&'a [String]>,
    allowed_patterns: Option<&'a [String]>,
    diagnostics: &'a mut Vec<Diagnostic>,
    // Stack to track what each branch node contributed (def_depth_inc, scope_depth_inc)
    stack: Vec<(bool, bool)>,
}

/// Check if a `defs` node (def with receiver) has an allowed receiver type.
/// RuboCop allows `def obj.method` when the receiver is a variable (local,
/// instance, class, global), a constant, or a method call. The `self` keyword
/// is NOT allowed — `def self.method` nested inside another def IS an offense.
fn is_allowed_receiver_node(node: ruby_prism::Node<'_>) -> bool {
    let node = util::unwrap_parentheses(node);
    // Variables: local, instance, class, global, implicit `it`
    if node.as_local_variable_read_node().is_some()
        || node.as_it_local_variable_read_node().is_some()
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
    // Method calls, including safe-navigation calls.
    if node.as_call_node().is_some() {
        return true;
    }
    false
}

fn has_allowed_receiver(def_node: &ruby_prism::DefNode<'_>) -> bool {
    def_node.receiver().is_some_and(is_allowed_receiver_node)
}

impl<'pr> Visit<'pr> for FullTreeWalker<'_> {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        let mut is_def = false;
        let mut is_scope = false;

        if node.as_def_node().is_some() {
            is_def = true;
        } else if node.as_singleton_class_node().is_some()
            || is_scope_creating_call(&node)
            || is_allowed_method_call(&node, self.allowed_methods, self.allowed_patterns)
        {
            is_scope = true;
        }

        // Check for offense BEFORE incrementing counters
        if is_def && self.def_depth > 0 && self.scope_depth == 0 {
            if let Some(def_node) = node.as_def_node() {
                if !has_allowed_receiver(&def_node) {
                    let offset = node.location().start_offset();
                    let (line, column) = self.source.offset_to_line_col(offset);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Method definitions must not be nested. Use `lambda` instead.".to_string(),
                    ));
                }
            }
        }

        if is_def {
            self.def_depth += 1;
        }
        if is_scope {
            self.scope_depth += 1;
        }
        self.stack.push((is_def, is_scope));
    }

    fn visit_branch_node_leave(&mut self) {
        if let Some((is_def, is_scope)) = self.stack.pop() {
            if is_def {
                self.def_depth -= 1;
            }
            if is_scope {
                self.scope_depth -= 1;
            }
        }
    }
}

/// Check if a node is a scope-creating call like Module.new, Class.new,
/// define_method, class_eval, etc. that creates a new method scope.
fn is_scope_creating_call(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    // Must have a block for defs inside to be in a new scope
    if call.block().is_none() {
        return false;
    }
    let method_name = call.name().as_slice();
    // Metaprogramming methods that create new scopes
    if matches!(
        method_name,
        b"define_method"
            | b"class_eval"
            | b"module_eval"
            | b"instance_eval"
            | b"class_exec"
            | b"module_exec"
            | b"instance_exec"
    ) {
        return true;
    }
    // Module.new, Class.new, Struct.new (also handles root-qualified like ::Module.new)
    if method_name == b"new" {
        if let Some(receiver) = call.receiver() {
            return crate::cop::shared::util::is_simple_constant(&receiver, b"Module")
                || crate::cop::shared::util::is_simple_constant(&receiver, b"Class")
                || crate::cop::shared::util::is_simple_constant(&receiver, b"Struct");
        }
    }
    // Data.define (Ruby 3.2+, recognized by rubocop-ast class_constructor?)
    if method_name == b"define" {
        if let Some(receiver) = call.receiver() {
            return crate::cop::shared::util::is_simple_constant(&receiver, b"Data");
        }
    }
    false
}

/// Check if a call node with a block matches AllowedMethods or AllowedPatterns.
/// This is used to treat such blocks as scope-creating (suppressing the offense).
fn is_allowed_method_call(
    node: &ruby_prism::Node<'_>,
    allowed_methods: Option<&[String]>,
    allowed_patterns: Option<&[String]>,
) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    // Must have a block
    if call.block().is_none() {
        return false;
    }
    let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
    if let Some(allowed) = allowed_methods {
        if allowed.iter().any(|m| m == method_name) {
            return true;
        }
    }
    if let Some(patterns) = allowed_patterns {
        if patterns.iter().any(|p| method_name.contains(p.as_str())) {
            return true;
        }
    }
    false
}

impl Cop for NestedMethodDefinition {
    fn name(&self) -> &'static str {
        "Lint/NestedMethodDefinition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allowed_methods = config.get_string_array("AllowedMethods");
        let allowed_patterns = config.get_string_array("AllowedPatterns");

        let root = parse_result.node();
        let mut walker = FullTreeWalker {
            source,
            cop: self,
            def_depth: 0,
            scope_depth: 0,
            allowed_methods: allowed_methods.as_deref(),
            allowed_patterns: allowed_patterns.as_deref(),
            diagnostics,
            stack: vec![],
        };
        walker.visit(&root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedMethodDefinition, "cops/lint/nested_method_definition");
}
