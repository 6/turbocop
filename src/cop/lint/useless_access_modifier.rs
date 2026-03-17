use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Lint/UselessAccessModifier — checks for redundant access modifiers.
///
/// ## Investigation findings
///
/// FP root causes (16 → 8 → 6 → 4 FPs):
/// - Original `check_scope` only handled top-level statements, while RuboCop's
///   `check_child_nodes` recursively propagates `(cur_vis, unused)` through all
///   non-scope child nodes. This caused FPs when access modifiers inside conditional
///   branches (e.g., `protected unless $TESTING`) changed visibility state, and FNs
///   when visibility leaked out of blocks.
/// - `class_eval`/`instance_eval` blocks inside `def` methods were incorrectly treated
///   as scopes. RuboCop's `macro?` / `in_macro_scope?` check means `private` inside
///   such blocks is not recognized as an access modifier.
/// - `ContextCreatingMethods` config was read but not used. Methods like `class_methods`
///   (from rubocop-rails plugin) must be treated as scope boundaries.
///
/// Fixes applied:
/// - Rewrote `check_scope` to use recursive `check_child_nodes` matching RuboCop's
///   architecture: propagates `(cur_vis, unused_modifier)` through all non-scope
///   child nodes, stopping at scope boundaries and `defs` nodes.
/// - Added `in_def` tracking to the visitor to skip `class_eval`/`instance_eval` blocks
///   nested inside method definitions (matching RuboCop's `macro?` gate).
/// - Implemented `ContextCreatingMethods` config: blocks calling configured methods
///   are treated as scope boundaries (e.g., `class_methods` from rubocop-rails).
/// - Added `is_new_scope` helper matching RuboCop's `start_of_new_scope?`.
/// - Added `visit_singleton_class_node` to handle `class << self` scopes.
/// - Added `is_bare_private_class_method` detection.
/// - Added `visit_program_node` for top-level access modifier detection.
/// - Added `module_function` to `AccessKind` and `get_access_modifier`.
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
        let context_creating = config
            .get_string_array("ContextCreatingMethods")
            .unwrap_or_default();
        let method_creating = config
            .get_string_array("MethodCreatingMethods")
            .unwrap_or_default();
        let mut visitor = UselessAccessVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            method_creating_methods: method_creating,
            context_creating_methods: context_creating,
            in_def: false,
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
    ModuleFunction,
}

impl AccessKind {
    fn as_str(self) -> &'static str {
        match self {
            AccessKind::Public => "public",
            AccessKind::Private => "private",
            AccessKind::Protected => "protected",
            AccessKind::ModuleFunction => "module_function",
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
        b"module_function" => Some(AccessKind::ModuleFunction),
        _ => None,
    }
}

/// Check if a call node is `private_class_method` without arguments (standalone statement).
fn is_bare_private_class_method(call: &ruby_prism::CallNode<'_>) -> bool {
    call.receiver().is_none()
        && call.arguments().is_none()
        && call.name().as_slice() == b"private_class_method"
}

/// Check if a call node is an access modifier or bare/args private_class_method.
/// Matches RuboCop's `access_modifier?` method.
fn is_access_modifier_or_private_class_method(call: &ruby_prism::CallNode<'_>) -> bool {
    get_access_modifier(call).is_some()
        || (call.receiver().is_none() && call.name().as_slice() == b"private_class_method")
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
/// Class/Module/Struct.new blocks, and ContextCreatingMethods blocks.
fn is_new_scope(node: &ruby_prism::Node<'_>, context_creating_methods: &[String]) -> bool {
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
            // ContextCreatingMethods (e.g., class_methods from rubocop-rails)
            if !context_creating_methods.is_empty() && call.receiver().is_none() {
                let name_str = std::str::from_utf8(name).unwrap_or("");
                if context_creating_methods.iter().any(|m| m == name_str) {
                    return true;
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

/// Check if a node is a singleton method def (def self.foo).
fn is_singleton_method_def(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(def_node) = node.as_def_node() {
        return def_node.receiver().is_some();
    }
    false
}

/// Recursively process child nodes, propagating `(cur_vis, unused_modifier)` state.
/// Matches RuboCop's `check_child_nodes` method:
/// - Access modifiers update `cur_vis` and `unused_modifier`
/// - Method definitions clear `unused_modifier`
/// - New scopes are processed independently (don't propagate state)
/// - `defs` nodes (singleton method defs) are skipped entirely
/// - All other nodes are recursed into, propagating state
#[allow(clippy::too_many_arguments)]
fn check_child_nodes<'pr>(
    cop: &UselessAccessModifier,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    node: &ruby_prism::Node<'pr>,
    mut cur_vis: AccessKind,
    mut unused_modifier: Option<(usize, AccessKind)>,
    method_creating_methods: &[String],
    context_creating_methods: &[String],
) -> (AccessKind, Option<(usize, AccessKind)>) {
    let children = collect_child_nodes(node);

    for child in &children {
        if let Some(call) = child.as_call_node() {
            if is_access_modifier_or_private_class_method(&call) {
                // Standalone private_class_method (no args) is always useless
                if is_bare_private_class_method(&call) {
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(cop.diagnostic(
                        source,
                        line,
                        column,
                        "Useless `private_class_method` access modifier.".to_string(),
                    ));
                    continue;
                }

                // private_class_method with arguments: in RuboCop, check_send_node
                // returns nil, which resets tracking state.
                if call.arguments().is_some() && call.name().as_slice() == b"private_class_method" {
                    unused_modifier = None;
                    continue;
                }

                if let Some(modifier_kind) = get_access_modifier(&call) {
                    if modifier_kind == cur_vis {
                        // Repeated modifier - always useless
                        let loc = call.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(cop.diagnostic(
                            source,
                            line,
                            column,
                            format!("Useless `{}` access modifier.", cur_vis.as_str()),
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
                        cur_vis = modifier_kind;
                        unused_modifier = Some((call.location().start_offset(), modifier_kind));
                    }
                    continue;
                }
            }
        }

        // Method definition clears the unused modifier
        if is_method_definition(child) || is_method_creating_call(child, method_creating_methods) {
            unused_modifier = None;
            continue;
        }

        // New scopes are checked independently — they don't propagate state
        if is_new_scope(child, context_creating_methods) {
            continue;
        }

        // Skip singleton method defs entirely (def self.foo)
        if is_singleton_method_def(child) {
            continue;
        }

        // For everything else, recurse and propagate state
        let result = check_child_nodes(
            cop,
            source,
            diagnostics,
            child,
            cur_vis,
            unused_modifier,
            method_creating_methods,
            context_creating_methods,
        );
        cur_vis = result.0;
        unused_modifier = result.1;
    }

    (cur_vis, unused_modifier)
}

/// Collect direct child nodes from a Prism node.
/// Since ruby_prism::Node doesn't have a generic child_nodes() method,
/// we handle each container type explicitly.
fn collect_child_nodes<'pr>(node: &ruby_prism::Node<'pr>) -> Vec<ruby_prism::Node<'pr>> {
    // StatementsNode — body of class/module/begin blocks
    if let Some(stmts) = node.as_statements_node() {
        return stmts.body().iter().collect();
    }
    // BlockNode — body of a block
    if let Some(block) = node.as_block_node() {
        if let Some(body) = block.body() {
            return collect_child_nodes(&body);
        }
        return Vec::new();
    }
    // IfNode
    if let Some(if_node) = node.as_if_node() {
        let mut children = Vec::new();
        // Don't include the condition — only the branches
        if let Some(stmts) = if_node.statements() {
            children.extend(stmts.body().iter());
        }
        if let Some(subsequent) = if_node.subsequent() {
            children.push(subsequent);
        }
        return children;
    }
    // UnlessNode
    if let Some(unless_node) = node.as_unless_node() {
        let mut children = Vec::new();
        if let Some(stmts) = unless_node.statements() {
            children.extend(stmts.body().iter());
        }
        if let Some(else_clause) = unless_node.else_clause() {
            children.push(else_clause.as_node());
        }
        return children;
    }
    // ElseNode
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            return stmts.body().iter().collect();
        }
        return Vec::new();
    }
    // BeginNode (explicit begin..end)
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            return stmts.body().iter().collect();
        }
        return Vec::new();
    }
    // CallNode — may have receiver, arguments, and a block
    if let Some(call) = node.as_call_node() {
        let mut children = Vec::new();
        if let Some(recv) = call.receiver() {
            children.push(recv);
        }
        if let Some(args) = call.arguments() {
            children.extend(args.arguments().iter());
        }
        if let Some(block) = call.block() {
            children.push(block);
        }
        return children;
    }
    // ParenthesesNode
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            return vec![body];
        }
        return Vec::new();
    }
    // LambdaNode
    if let Some(lambda) = node.as_lambda_node() {
        if let Some(body) = lambda.body() {
            return vec![body];
        }
        return Vec::new();
    }
    // CaseNode
    if let Some(case_node) = node.as_case_node() {
        let mut children: Vec<ruby_prism::Node<'pr>> = Vec::new();
        children.extend(case_node.conditions().iter());
        if let Some(else_clause) = case_node.else_clause() {
            children.push(else_clause.as_node());
        }
        return children;
    }
    // WhenNode
    if let Some(when_node) = node.as_when_node() {
        if let Some(stmts) = when_node.statements() {
            return stmts.body().iter().collect();
        }
        return Vec::new();
    }
    Vec::new()
}

fn check_scope(
    cop: &UselessAccessModifier,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    stmts: &ruby_prism::StatementsNode<'_>,
    method_creating_methods: &[String],
    context_creating_methods: &[String],
) {
    let stmts_node = stmts.as_node();
    let (_, unused_modifier) = check_child_nodes(
        cop,
        source,
        diagnostics,
        &stmts_node,
        AccessKind::Public,
        None,
        method_creating_methods,
        context_creating_methods,
    );

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
    context_creating_methods: Vec<String>,
    /// Track whether we are inside a def/defs node.
    /// class_eval/instance_eval blocks inside defs should not be treated as scopes
    /// because RuboCop's `macro?` check means access modifiers inside them are not
    /// recognized as bare access modifiers.
    in_def: bool,
}

/// Check if a call node is a bare access modifier (including module_function and
/// private_class_method without args). Used for top-level detection.
fn is_access_modifier_call(call: &ruby_prism::CallNode<'_>) -> bool {
    get_access_modifier(call).is_some() || is_bare_private_class_method(call)
}

impl<'pr> Visit<'pr> for UselessAccessVisitor<'_, '_> {
    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        // Top-level access modifiers are always useless (RuboCop's on_begin handler).
        let stmts = node.statements();
        for stmt in stmts.body().iter() {
            if let Some(call) = stmt.as_call_node() {
                if is_access_modifier_call(&call) {
                    let loc = call.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    let name = if is_bare_private_class_method(&call) {
                        "private_class_method".to_string()
                    } else {
                        get_access_modifier(&call).unwrap().as_str().to_string()
                    };
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        format!("Useless `{}` access modifier.", name),
                    ));
                }
            }
        }
        ruby_prism::visit_program_node(self, node);
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                check_scope(
                    self.cop,
                    self.source,
                    &mut self.diagnostics,
                    &stmts,
                    &self.method_creating_methods,
                    &self.context_creating_methods,
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
                    &self.context_creating_methods,
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
                    &self.context_creating_methods,
                );
            }
        }
        ruby_prism::visit_singleton_class_node(self, node);
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let was_in_def = self.in_def;
        self.in_def = true;
        ruby_prism::visit_def_node(self, node);
        self.in_def = was_in_def;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Handle eval blocks (class_eval, instance_eval) and constructor blocks
        // (Class.new, Module.new, Struct.new, Data.define) as scopes.
        // Skip if we're inside a def method — RuboCop's macro? check means
        // access modifiers inside class_eval blocks in defs are not recognized.
        if !self.in_def {
            if let Some(block_node) = node.block() {
                if let Some(block) = block_node.as_block_node() {
                    let name = node.name().as_slice();
                    let is_eval_scope = if name == b"class_eval" || name == b"instance_eval" {
                        true
                    } else if name == b"new" || name == b"define" {
                        node.receiver()
                            .as_ref()
                            .is_some_and(|r| is_class_constructor_receiver(r))
                    } else {
                        false
                    };
                    if is_eval_scope {
                        if let Some(body) = block.body() {
                            if let Some(stmts) = body.as_statements_node() {
                                check_scope(
                                    self.cop,
                                    self.source,
                                    &mut self.diagnostics,
                                    &stmts,
                                    &self.method_creating_methods,
                                    &self.context_creating_methods,
                                );
                            }
                        }
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessAccessModifier, "cops/lint/useless_access_modifier");
}
