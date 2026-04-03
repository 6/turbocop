use crate::cop::shared::access_modifier_predicates;
use crate::cop::shared::method_dispatch_predicates;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Lint/UselessAccessModifier — checks for redundant access modifiers.
///
/// ## Investigation findings
///
/// FP root causes (16 → 8 → 6 → 4 → 0 FPs):
/// - Original `check_scope` only handled top-level statements, while RuboCop's
///   `check_child_nodes` recursively propagates `(cur_vis, unused)` through all
///   non-scope child nodes. This caused FPs when access modifiers inside conditional
///   branches (e.g., `protected unless $TESTING`) changed visibility state, and FNs
///   when visibility leaked out of blocks.
/// - `class_eval`/`instance_eval` blocks inside `def` methods were incorrectly treated
///   as scopes. RuboCop's `macro?` / `in_macro_scope?` check means `private` inside
///   such blocks is not recognized as an access modifier.
/// - The `in_def` guard later became too broad and skipped *all* scope-creating
///   call blocks inside methods. RuboCop only suppresses `class_eval` /
///   `instance_eval` macro scopes there; constructor blocks like `Class.new do`
///   must still be analyzed, or useless `private` before singleton defs is missed.
/// - `private_class_method :foo` with arguments also resets RuboCop's visibility
///   tracking for later instance-method modifiers. Keeping the previous `cur_vis`
///   caused synthetic false positives where a later `private`/`public`/`protected`
///   was treated as repeated even though RuboCop accepts it.
/// - `ContextCreatingMethods` config was read but not used. Methods like `class_methods`
///   (from rubocop-rails plugin) must be treated as scope boundaries.
/// - Chained method calls on access modifiers (e.g., `private.should equal(nil)`)
///   were incorrectly treated as bare access modifiers because `collect_child_nodes`
///   for CallNode returned the receiver. In RuboCop, `in_macro_scope?` rejects access
///   modifiers whose parent is a send node. Fixed by tracking `in_call_children` flag
///   that disables access modifier detection when recursing into CallNode receiver/args.
/// - Single-statement class/module bodies (e.g., `module Foo; describe do...end; end`)
///   triggered `check_scope` even though in the Parser AST this body is NOT a begin
///   node. RuboCop's `check_node` only calls `check_scope` on begin-type bodies.
///   Fixed by adding `check_body` that only runs `check_scope` for multi-statement bodies.
///
/// Fixes applied:
/// - Rewrote `check_scope` to use recursive `check_child_nodes` matching RuboCop's
///   architecture: propagates `(cur_vis, unused_modifier)` through all non-scope
///   child nodes, stopping at scope boundaries and `defs` nodes.
/// - Added `in_def` tracking to the visitor to skip `class_eval`/`instance_eval` blocks
///   nested inside method definitions (matching RuboCop's `macro?` gate).
/// - Narrowed that `in_def` skip so it only applies to `class_eval`/`instance_eval`.
///   `Class.new`/`Module.new`/`Struct.new` blocks and configured context-creating
///   blocks inside methods are now still checked.
/// - Reset visibility to `public` after `private_class_method` with arguments,
///   matching RuboCop's effective state reset for subsequent instance-method
///   access modifiers.
/// - Implemented `ContextCreatingMethods` config: blocks calling configured methods
///   are treated as scope boundaries (e.g., `class_methods` from rubocop-rails).
/// - Added `is_new_scope` helper matching RuboCop's `start_of_new_scope?`.
/// - Added `visit_singleton_class_node` to handle `class << self` scopes.
/// - Added `is_bare_private_class_method` detection.
/// - Added `visit_program_node` for top-level access modifier detection.
/// - Added `module_function` to `AccessKind` and `get_access_modifier`.
/// - Added `in_call_children` flag to `check_child_nodes` to disable access modifier
///   detection when recursing into CallNode receiver/arguments (matching `in_macro_scope?`).
/// - Added `check_body` to replicate Parser's begin-only scope check for multi-statement bodies.
/// - Expanded `is_method_definition` to recognize inline access modifiers (`public def foo`,
///   `private def foo`) and method decorators (`memoize def foo`, `override def foo`) as
///   method definitions. In Prism, these are CallNodes with a DefNode argument. RuboCop handles
///   this by recursing into child nodes of the send node, finding the inner `def`. We handle it
///   by checking CallNode arguments for DefNode in `is_method_definition`.
/// - Config-aware scope bug: `ContextCreatingMethods` blocks (notably `included` from
///   `rubocop-rails`) were marked as new scopes by `is_new_scope`, but `visit_call_node`
///   only analyzed `class_eval`/constructor blocks. In corpus runs this skipped
///   `included do ... end` bodies entirely, producing false negatives for useless
///   `private` before singleton defs. Fixed by checking configured context-creating
///   blocks as separate scopes in `visit_call_node` too.
/// - `private_class_method` with arguments incorrectly reset `cur_vis` to `Public`.
///   In RuboCop, `check_send_node` returns `nil` for this case, setting `cur_vis` to
///   `nil` (unknown). This meant a later `public` after `private` + instance methods +
///   `private_class_method :foo` was treated as `Public == Public` (repeated/useless)
///   instead of a new visibility change. Fixed by using `Option<AccessKind>` for
///   `cur_vis` and setting it to `None` after `private_class_method` with args.
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
    if !access_modifier_predicates::is_bare_access_modifier(call) {
        return None;
    }
    match call.name().as_slice() {
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
        || method_dispatch_predicates::is_command(call, b"private_class_method")
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
            // Inline access modifiers (`public def foo`, `private def foo`, `protected def foo`)
            // and method decorators (`memoize def foo`, `override def foo`) — any call with a
            // non-singleton def argument counts as a method definition.
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    if let Some(def_node) = arg.as_def_node() {
                        if def_node.receiver().is_none() {
                            return true;
                        }
                    }
                }
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
///
/// The `in_call_children` flag tracks whether we are processing sub-expressions of a
/// CallNode (receiver, arguments) vs. direct scope-level statements. Access modifiers
/// are only recognized when `in_call_children` is false, matching RuboCop's `macro?` /
/// `in_macro_scope?` check which requires the node's parent to be a scope-like container
/// (begin, block, if, class, module) rather than a send node.
#[allow(clippy::too_many_arguments)]
fn check_child_nodes<'pr>(
    cop: &UselessAccessModifier,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    node: &ruby_prism::Node<'pr>,
    mut cur_vis: Option<AccessKind>,
    mut unused_modifier: Option<(usize, AccessKind)>,
    method_creating_methods: &[String],
    context_creating_methods: &[String],
    in_call_children: bool,
) -> (Option<AccessKind>, Option<(usize, AccessKind)>) {
    // If the node itself is a CallNode, handle its children directly.
    // collect_child_nodes returns empty for CallNode, so we must process
    // receiver/args/block here before falling through to the loop.
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            let result = check_child_nodes(
                cop,
                source,
                diagnostics,
                &recv,
                cur_vis,
                unused_modifier,
                method_creating_methods,
                context_creating_methods,
                in_call_children,
            );
            cur_vis = result.0;
            unused_modifier = result.1;
        }
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                let result = check_child_nodes(
                    cop,
                    source,
                    diagnostics,
                    &arg,
                    cur_vis,
                    unused_modifier,
                    method_creating_methods,
                    context_creating_methods,
                    in_call_children,
                );
                cur_vis = result.0;
                unused_modifier = result.1;
            }
        }
        if let Some(block) = call.block() {
            let result = check_child_nodes(
                cop,
                source,
                diagnostics,
                &block,
                cur_vis,
                unused_modifier,
                method_creating_methods,
                context_creating_methods,
                false, // block bodies reset in_call_children
            );
            cur_vis = result.0;
            unused_modifier = result.1;
        }
        return (cur_vis, unused_modifier);
    }

    let children = collect_child_nodes(node);

    for child in &children {
        // Only check for access modifiers when NOT inside a CallNode's receiver/arguments.
        if !in_call_children {
            if let Some(call) = child.as_call_node() {
                if is_access_modifier_or_private_class_method(&call) {
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

                    if call.arguments().is_some()
                        && call.name().as_slice() == b"private_class_method"
                    {
                        // In RuboCop, check_send_node returns nil for private_class_method
                        // with args, setting cur_vis to nil (unknown state). This means a
                        // subsequent access modifier is always treated as a new change.
                        cur_vis = None;
                        unused_modifier = None;
                        continue;
                    }

                    if let Some(modifier_kind) = get_access_modifier(&call) {
                        if Some(modifier_kind) == cur_vis {
                            let loc = call.location();
                            let (line, column) = source.offset_to_line_col(loc.start_offset());
                            diagnostics.push(cop.diagnostic(
                                source,
                                line,
                                column,
                                format!("Useless `{}` access modifier.", modifier_kind.as_str()),
                            ));
                        } else {
                            if let Some((offset, old_vis)) = unused_modifier {
                                let (line, column) = source.offset_to_line_col(offset);
                                diagnostics.push(cop.diagnostic(
                                    source,
                                    line,
                                    column,
                                    format!("Useless `{}` access modifier.", old_vis.as_str()),
                                ));
                            }
                            cur_vis = Some(modifier_kind);
                            unused_modifier = Some((call.location().start_offset(), modifier_kind));
                        }
                        continue;
                    }
                }
            }
        }

        // Method definition clears the unused modifier
        if is_method_definition(child) || is_method_creating_call(child, method_creating_methods) {
            unused_modifier = None;
            continue;
        }

        // New scopes are checked independently
        if is_new_scope(child, context_creating_methods) {
            continue;
        }

        // Skip singleton method defs entirely (def self.foo)
        if is_singleton_method_def(child) {
            continue;
        }

        // For CallNode children, recurse into receiver/arguments with access modifiers
        // disabled (in_call_children=true) but into the block with them enabled.
        if let Some(call) = child.as_call_node() {
            if let Some(recv) = call.receiver() {
                let result = check_child_nodes(
                    cop,
                    source,
                    diagnostics,
                    &recv,
                    cur_vis,
                    unused_modifier,
                    method_creating_methods,
                    context_creating_methods,
                    true,
                );
                cur_vis = result.0;
                unused_modifier = result.1;
            }
            if let Some(args) = call.arguments() {
                for arg in args.arguments().iter() {
                    let result = check_child_nodes(
                        cop,
                        source,
                        diagnostics,
                        &arg,
                        cur_vis,
                        unused_modifier,
                        method_creating_methods,
                        context_creating_methods,
                        true,
                    );
                    cur_vis = result.0;
                    unused_modifier = result.1;
                }
            }
            if let Some(block) = call.block() {
                let result = check_child_nodes(
                    cop,
                    source,
                    diagnostics,
                    &block,
                    cur_vis,
                    unused_modifier,
                    method_creating_methods,
                    context_creating_methods,
                    false,
                );
                cur_vis = result.0;
                unused_modifier = result.1;
            }
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
            in_call_children,
        );
        cur_vis = result.0;
        unused_modifier = result.1;
    }

    (cur_vis, unused_modifier)
}

/// Collect direct child nodes from a Prism node.
fn collect_child_nodes<'pr>(node: &ruby_prism::Node<'pr>) -> Vec<ruby_prism::Node<'pr>> {
    if let Some(stmts) = node.as_statements_node() {
        return stmts.body().iter().collect();
    }
    if let Some(block) = node.as_block_node() {
        if let Some(body) = block.body() {
            return collect_child_nodes(&body);
        }
        return Vec::new();
    }
    if let Some(if_node) = node.as_if_node() {
        let mut children = Vec::new();
        if let Some(stmts) = if_node.statements() {
            children.extend(stmts.body().iter());
        }
        if let Some(subsequent) = if_node.subsequent() {
            children.push(subsequent);
        }
        return children;
    }
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
    if let Some(else_node) = node.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            return stmts.body().iter().collect();
        }
        return Vec::new();
    }
    if let Some(begin_node) = node.as_begin_node() {
        if let Some(stmts) = begin_node.statements() {
            return stmts.body().iter().collect();
        }
        return Vec::new();
    }
    // CallNode — handled explicitly in check_child_nodes
    if node.as_call_node().is_some() {
        return Vec::new();
    }
    if let Some(paren) = node.as_parentheses_node() {
        if let Some(body) = paren.body() {
            return vec![body];
        }
        return Vec::new();
    }
    if let Some(lambda) = node.as_lambda_node() {
        if let Some(body) = lambda.body() {
            return vec![body];
        }
        return Vec::new();
    }
    if let Some(case_node) = node.as_case_node() {
        let mut children: Vec<ruby_prism::Node<'pr>> = Vec::new();
        children.extend(case_node.conditions().iter());
        if let Some(else_clause) = case_node.else_clause() {
            children.push(else_clause.as_node());
        }
        return children;
    }
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
        Some(AccessKind::Public),
        None,
        method_creating_methods,
        context_creating_methods,
        false,
    );

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

/// Replicate RuboCop's `check_node` logic for class/module/sclass bodies.
///
/// In the Parser gem, a single-statement body is the node itself (not wrapped in `begin`).
/// RuboCop's `check_node` calls `check_scope` only when the body is `begin_type?` (multiple
/// statements). For a single statement, it only flags if it's a bare access modifier.
///
/// Prism always wraps bodies in `StatementsNode`, so we replicate this distinction here:
/// - 1 statement: only flag if it's a bare access modifier (no `check_scope`)
/// - 2+ statements: call `check_scope` (full analysis)
fn check_body(
    cop: &UselessAccessModifier,
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    stmts: &ruby_prism::StatementsNode<'_>,
    method_creating_methods: &[String],
    context_creating_methods: &[String],
) {
    let body = stmts.body();
    if body.len() == 1 {
        let single = body.iter().next().unwrap();
        if let Some(call) = single.as_call_node() {
            if let Some(kind) = get_access_modifier(&call) {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    format!("Useless `{}` access modifier.", kind.as_str()),
                ));
            } else if is_bare_private_class_method(&call) {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Useless `private_class_method` access modifier.".to_string(),
                ));
            }
        }
    } else {
        check_scope(
            cop,
            source,
            diagnostics,
            stmts,
            method_creating_methods,
            context_creating_methods,
        );
    }
}

struct UselessAccessVisitor<'a, 'src> {
    cop: &'a UselessAccessModifier,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    method_creating_methods: Vec<String>,
    context_creating_methods: Vec<String>,
    in_def: bool,
}

fn is_access_modifier_call(call: &ruby_prism::CallNode<'_>) -> bool {
    get_access_modifier(call).is_some() || is_bare_private_class_method(call)
}

impl<'pr> Visit<'pr> for UselessAccessVisitor<'_, '_> {
    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
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
                check_body(
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
                check_body(
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
                check_body(
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
        if let Some(block_node) = node.block() {
            if let Some(block) = block_node.as_block_node() {
                let name = node.name().as_slice();
                let is_eval_macro_scope = name == b"class_eval" || name == b"instance_eval";
                let is_constructor_scope = (name == b"new" || name == b"define")
                    && node
                        .receiver()
                        .as_ref()
                        .is_some_and(|r| is_class_constructor_receiver(r));
                let is_context_scope =
                    if !self.context_creating_methods.is_empty() && node.receiver().is_none() {
                        let name_str = std::str::from_utf8(name).unwrap_or("");
                        self.context_creating_methods.iter().any(|m| m == name_str)
                    } else {
                        false
                    };
                let should_check_scope = if self.in_def {
                    !is_eval_macro_scope && (is_constructor_scope || is_context_scope)
                } else {
                    is_eval_macro_scope || is_constructor_scope || is_context_scope
                };

                if should_check_scope {
                    if let Some(body) = block.body() {
                        if let Some(stmts) = body.as_statements_node() {
                            check_body(
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
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yml::Value;

    fn context_creating_methods_config(methods: &[&str]) -> CopConfig {
        let mut config = CopConfig::default();
        config.options.insert(
            "ContextCreatingMethods".to_string(),
            Value::Sequence(
                methods
                    .iter()
                    .map(|method| Value::String((*method).to_string()))
                    .collect(),
            ),
        );
        config
    }

    crate::cop_fixture_tests!(UselessAccessModifier, "cops/lint/useless_access_modifier");

    #[test]
    fn offense_in_included_block_with_context_creating_methods_config() {
        let fixture = b"module WithIncludedSingletonMethod\n  extend ActiveSupport::Concern\n\n  included do\n    private\n    ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.\n\n    def self.singleton_method_added(method_name)\n      method_name\n    end\n  end\nend\n";
        crate::testutil::assert_cop_offenses_full_with_config(
            &UselessAccessModifier,
            fixture,
            context_creating_methods_config(&["included"]),
        );
    }

    #[test]
    fn offense_after_singleton_def_in_included_block_with_context_creating_methods_config() {
        let fixture = b"module WithIncludedSingletonMethodsAroundPrivate\n  SOME_CONSTANT = 42\n\n  included do\n    def self.method_missing(name, *)\n      name\n    end\n\n    private\n    ^^^^^^^ Lint/UselessAccessModifier: Useless `private` access modifier.\n\n    def self.all_types\n      []\n    end\n  end\nend\n";
        crate::testutil::assert_cop_offenses_full_with_config(
            &UselessAccessModifier,
            fixture,
            context_creating_methods_config(&["included"]),
        );
    }
}
