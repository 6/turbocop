use ruby_prism::Visit;

use crate::cop::shared::method_dispatch_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/StaticClass: Prefer modules to classes with only class methods.
///
/// Checks for classes that contain only class-level methods and could be
/// converted to modules. Matches RuboCop's logic which allows:
/// - `def self.method` (public class methods)
/// - `class << self` blocks (only if all children are public defs or assignments)
/// - Constant/variable assignments (`CONST = 1`)
/// - `extend` calls
///
/// Does NOT flag classes that contain:
/// - Instance methods (`def foo`)
/// - `include`/`prepend` calls
/// - Macro-style method calls
/// - Private/protected methods (either via visibility modifiers or inside `class << self`)
/// - A superclass (`class C < Base`)
/// - Empty bodies
///
/// Root causes of historical FPs:
/// - `class << self` with `private` or macro calls (e.g. `attr_accessor`) was
///   blindly accepted; now we validate sclass children are all public defs/assignments.
///
/// Root causes of historical FNs:
/// - Constant assignments (`CONST = 1`) were rejected as "other node types";
///   now allowed per RuboCop's `equals_asgn?` check.
/// - `extend` calls were rejected; now allowed per RuboCop's `extend_call?`.
/// - `class << self` with only public defs/assignments wasn't properly validated.
/// - Empty `class << self; end` blocks caused `is_convertible_sclass` to return false
///   (early-exit on empty children), but Ruby's `[].all?` returns true so these are
///   convertible. Fixed by removing the empty-children guard.
/// - Multi-assignment (`A, B = expr`) was not recognized by `is_assignment`; added
///   `MultiWriteNode` to match RuboCop's `equals_asgn?` which includes `masgn`.
pub struct StaticClass;

impl Cop for StaticClass {
    fn name(&self) -> &'static str {
        "Style/StaticClass"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = StaticClassVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct StaticClassVisitor<'a> {
    cop: &'a StaticClass,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for StaticClassVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // Classes with a parent class cannot safely be converted to modules
        if node.superclass().is_some() {
            ruby_prism::visit_class_node(self, node);
            return;
        }

        if class_convertible_to_module(node) {
            let (line, column) = self
                .source
                .offset_to_line_col(node.location().start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Prefer modules to classes with only class methods.".to_string(),
            ));
        }

        ruby_prism::visit_class_node(self, node);
    }
}

/// Extract child nodes from a class or singleton class body.
fn class_elements<'pr>(body: Option<ruby_prism::Node<'pr>>) -> Vec<ruby_prism::Node<'pr>> {
    match body {
        None => vec![],
        Some(node) => {
            if let Some(stmts) = node.as_statements_node() {
                stmts.body().iter().collect()
            } else {
                // Single-expression body (no StatementsNode wrapper)
                vec![node]
            }
        }
    }
}

/// Check if a class can be converted to a module (RuboCop compatibility).
/// Requires non-empty body where every child is one of:
/// 1. A public `def self.method` (defs with receiver)
/// 2. A convertible `class << self` (all children are public defs or assignments)
/// 3. A constant/variable assignment
/// 4. An `extend` call
fn class_convertible_to_module(class_node: &ruby_prism::ClassNode<'_>) -> bool {
    let nodes = class_elements(class_node.body());
    if nodes.is_empty() {
        return false;
    }

    // Track visibility state: bare `private`/`protected` modifiers change
    // the visibility of subsequent methods. However, they are send nodes
    // and won't match any allowed node type, so they naturally cause
    // `all()` to return false. We don't need explicit tracking.
    nodes.iter().all(|node| {
        is_public_class_method(node)
            || is_convertible_sclass(node)
            || is_assignment(node)
            || is_extend_call(node)
    })
}

/// Check if node is a `def self.method` (class method with receiver).
fn is_public_class_method(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(def) = node.as_def_node() {
        def.receiver().is_some()
    } else {
        false
    }
}

/// Check if a `class << self` block is convertible (all children are
/// public defs or assignments, no macro calls or visibility modifiers).
fn is_convertible_sclass(node: &ruby_prism::Node<'_>) -> bool {
    let Some(sclass) = node.as_singleton_class_node() else {
        return false;
    };

    let children = class_elements(sclass.body());

    // Empty `class << self; end` is a no-op and convertible (Ruby's `[].all?` is true)
    children.iter().all(|child| {
        // Inside class << self, regular defs (no receiver) are class methods
        child.as_def_node().is_some_and(|d| d.receiver().is_none()) || is_assignment(child)
    })
}

/// Check if node is an assignment (constant, local var, ivar, cvar, gvar).
/// Matches RuboCop's `equals_asgn?`.
fn is_assignment(node: &ruby_prism::Node<'_>) -> bool {
    node.as_constant_write_node().is_some()
        || node.as_constant_path_write_node().is_some()
        || node.as_local_variable_write_node().is_some()
        || node.as_instance_variable_write_node().is_some()
        || node.as_class_variable_write_node().is_some()
        || node.as_global_variable_write_node().is_some()
        || node.as_multi_write_node().is_some()
}

/// Check if node is an `extend` call.
fn is_extend_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        method_dispatch_predicates::is_command(&call, b"extend")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StaticClass, "cops/style/static_class");
}
