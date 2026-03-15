use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Lint/UselessAccessModifier — checks for redundant access modifiers.
///
/// ## Investigation findings
///
/// FP root causes (16 FPs):
/// - `has_method_definition_in_subtree` only recursed into `if`/`unless` nodes, missing
///   `define_method` calls inside `.each` blocks, `begin..end` blocks, and other containers.
///   RuboCop's `check_child_nodes` recurses into ALL non-scope, non-defs child nodes.
///
/// Fixes applied:
/// - Rewrote `has_method_definition_in_subtree` to recursively traverse all relevant
///   container types (blocks, begin, call arguments, parentheses, else clauses) while
///   stopping at scope boundaries (class, module, sclass, class_eval/instance_eval blocks,
///   Class/Module/Struct.new blocks).
/// - Added `is_new_scope` helper matching RuboCop's `start_of_new_scope?`.
/// - Added `visit_singleton_class_node` to handle `class << self` scopes.
pub struct UselessAccessModifier;

impl Cop for UselessAccessModifier {
    fn name(&self) -> &'static str {
        "Lint/UselessAccessModifier"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _context_creating = config.get_string_array("ContextCreatingMethods");
        let method_creating = config
            .get_string_array("MethodCreatingMethods")
            .unwrap_or_default();
        let mut visitor = UselessAccessVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            method_creating_methods: method_creating,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessKind {
    Public,
    Private,
    Protected,
}

impl AccessKind {
    fn as_str(self) -> &'static str {
        match self {
            AccessKind::Public => "public",
            AccessKind::Private => "private",
            AccessKind::Protected => "protected",
        }
    }
}

fn get_access_modifier(call: &ruby_prism::CallNode<'_>) -> Option<AccessKind> {
    if call.receiver().is_some() || call.arguments().is_some() {
        return None;
    }
    let name = call.name().as_slice();
    match name {
        b"public" => Some(AccessKind::Public),
        b"private" => Some(AccessKind::Private),
        b"protected" => Some(AccessKind::Protected),
        _ => None,
    }
}

fn is_method_definition(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(def_node) = node.as_def_node() {
        // Singleton methods (def self.foo) are NOT affected by access modifiers,
        // so they don't count as method definitions for our purposes.
        if def_node.receiver().is_none() {
            return true;
        }
        return false;
    }
    // attr_reader/writer/accessor or define_method as a bare call
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() {
            let name = call.name().as_slice();
            if name == b"attr_reader"
                || name == b"attr_writer"
                || name == b"attr_accessor"
                || name == b"attr"
                || name == b"define_method"
            {
                return true;
            }
        }
    }
    false
}

/// Check if a node is a call to one of the configured MethodCreatingMethods.
fn is_method_creating_call(
    node: &ruby_prism::Node<'_>,
    method_creating_methods: &[String],
) -> bool {
    if method_creating_methods.is_empty() {
        return false;
    }
    if let Some(call) = node.as_call_node() {
        if call.receiver().is_none() {
            let name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
            return method_creating_methods.iter().any(|m| m == name);
        }
    }
    false
}

/// Check if a node is a new scope boundary where access modifier tracking resets.
/// Matches RuboCop's `start_of_new_scope?`: class, module, sclass, class_eval/instance_eval blocks,
/// and Class/Module/Struct.new blocks.
fn is_new_scope(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_class_node().is_some()
        || node.as_module_node().is_some()
        || node.as_singleton_class_node().is_some()
    {
        return true;
    }
    // class_eval/instance_eval blocks and Class/Module/Struct.new blocks
    if let Some(call) = node.as_call_node() {
        if call.block().is_some() {
            let name = call.name().as_slice();
            if name == b"class_eval" || name == b"instance_eval" {
                return true;
            }
            // Class.new, Module.new, Struct.new, ::Class.new, etc.
            if name == b"new" {
                if let Some(recv) = call.receiver() {
                    if is_class_constructor_receiver(&recv) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Check if a receiver node is Class, Module, Struct, or their ::prefixed variants.
fn is_class_constructor_receiver(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(const_read) = node.as_constant_read_node() {
        let name = const_read.name().as_slice();
        return name == b"Class" || name == b"Module" || name == b"Struct" || name == b"Data";
    }
    if let Some(const_path) = node.as_constant_path_node() {
        // ::Class, ::Module, ::Struct, ::Data
        if const_path.parent().is_none() {
            if let Some(name_node) = const_path.name() {
                let name = name_node.as_slice();
                return name == b"Class"
                    || name == b"Module"
                    || name == b"Struct"
                    || name == b"Data";
            }
        }
    }
    false
}

/// Recursively check if a node or any of its descendants contain a method definition.
/// Mirrors RuboCop's `check_child_nodes` recursion logic:
/// - Stop at new scopes (class, module, sclass, eval blocks)
/// - Skip `defs` (singleton method defs) entirely
/// - Recurse into all other child nodes (blocks, if/unless, begin, etc.)
fn has_method_definition_in_subtree(
    node: &ruby_prism::Node<'_>,
    method_creating_methods: &[String],
) -> bool {
    if is_method_definition(node) || is_method_creating_call(node, method_creating_methods) {
        return true;
    }
    // Don't recurse into singleton method defs (def self.foo) — they are skipped entirely
    if node.as_def_node().is_some() {
        return false;
    }
    // Don't recurse into new scopes
    if is_new_scope(node) {
        return false;
    }
    // Recurse into child nodes of known container types.
    // ruby_prism::Node doesn't have a generic child_nodes() method,
    // so we handle each container type that can appear in a class/module body.
    recurse_children(node, method_creating_methods)
}

/// Recurse into children of known container types looking for method definitions.
fn recurse_children(node: &ruby_prism::Node<'_>, method_creating_methods: &[String]) -> bool {
    // StatementsNode — body of begin blocks, etc.
    if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            if has_method_definition_in_subtree(&child, method_creating_methods) {
                return true;
            }
        }
        return false;
    }
    // CallNode — may have arguments and a block
    if let Some(call) = node.as_call_node() {
        // Check arguments (e.g., `helper_method def foo; end`)
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if has_method_definition_in_subtree(&arg, method_creating_methods) {
                    return true;
                }
            }
        }
        // Check block body (e.g., `[1,2].each do |i| define_method(...) end`)
        if let Some(block) = call.block() {
            if has_method_definition_in_subtree(&block, method_creating_methods) {
                return true;
            }
        }
        return false;
    }
    // BlockNode — body of a block
    if let Some(block) = node.as_block_node() {
        if let Some(body) = block.body() {
            if has_method_definition_in_subtree(&body, method_creating_methods) {
                return true;
            }
        }
        return false;
    }
    // IfNode
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for stmt in stmts.body().iter() {
                if has_method_definition_in_subtree(&stmt, method_creating_methods) {
                    return true;
                }
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            if has_method_definition_in_subtree(&subsequent, method_creating_methods) {
                return true;
            }
        }
        return false;
    }
    // UnlessNode
    if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            for stmt in stmts.body().iter() {
                if has_method_definition_in_subtree(&stmt, method_creating_methods) {
                    return true;
                }
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if has_method_definition_in_subtree(&else_clause.as_node(), method_creating_methods) {
                return true;
            }
        }
        return false;
    }
    // ElseNode
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            for stmt in stmts.body().iter() {
                if has_method_definition_in_subtree(&stmt, method_creating_methods) {
                    return true;
                }
            }
        }
        return false;
    }
    // BeginNode (explicit begin..end)
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            for stmt in stmts.body().iter() {
                if has_method_definition_in_subtree(&stmt, method_creating_methods) {
                    return true;
                }
            }
        }
        return false;
    }
    // ParenthesesNode
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            if has_method_definition_in_subtree(&body, method_creating_methods) {
                return true;
            }
        }
        return false;
    }
    false
}

fn check_scope(
    cop: &UselessAccessModifier,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    stmts: &ruby_prism::StatementsNode<'_>,
    method_creating_methods: &[String],
) {
    let body: Vec<_> = stmts.body().iter().collect();

    let mut current_vis = AccessKind::Public;
    let mut unused_modifier: Option<(usize, AccessKind)> = None;

    for stmt in &body {
        if let Some(call) = stmt.as_call_node() {
            if let Some(modifier_kind) = get_access_modifier(&call) {
                if modifier_kind == current_vis {
                    // Repeated modifier - always useless
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(cop.diagnostic(
                        source,
                        line,
                        column,
                        format!("Useless `{}` access modifier.", current_vis.as_str()),
                    ));
                } else {
                    // New modifier - flag previous if unused
                    if let Some((offset, old_vis)) = unused_modifier {
                        let (line, column) = source.offset_to_line_col(offset);
                        diagnostics.push(cop.diagnostic(
                            source,
                            line,
                            column,
                            format!("Useless `{}` access modifier.", old_vis.as_str()),
                        ));
                    }
                    current_vis = modifier_kind;
                    unused_modifier = Some((call.location().start_offset(), modifier_kind));
                }
                continue;
            }
        }

        if has_method_definition_in_subtree(stmt, method_creating_methods) {
            unused_modifier = None;
        }
    }

    // If the last modifier was never followed by a method definition
    if let Some((offset, vis)) = unused_modifier {
        let (line, column) = source.offset_to_line_col(offset);
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            format!("Useless `{}` access modifier.", vis.as_str()),
        ));
    }
}

struct UselessAccessVisitor<'a, 'src> {
    cop: &'a UselessAccessModifier,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    method_creating_methods: Vec<String>,
}

impl<'pr> Visit<'pr> for UselessAccessVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                check_scope(
                    self.cop,
                    self.source,
                    &mut self.diagnostics,
                    &stmts,
                    &self.method_creating_methods,
                );
            }
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                check_scope(
                    self.cop,
                    self.source,
                    &mut self.diagnostics,
                    &stmts,
                    &self.method_creating_methods,
                );
            }
        }
        ruby_prism::visit_module_node(self, node);
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                check_scope(
                    self.cop,
                    self.source,
                    &mut self.diagnostics,
                    &stmts,
                    &self.method_creating_methods,
                );
            }
        }
        ruby_prism::visit_singleton_class_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessAccessModifier, "cops/lint/useless_access_modifier");
}
