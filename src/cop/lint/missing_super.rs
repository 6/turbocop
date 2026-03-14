use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-11)
///
/// Corpus oracle reported FP=73, FN=43.
///
/// FP=73: `is_inside_class_with_stateful_parent()` walked the full class_stack
/// looking for any `Class.new(Parent)` ancestor, but RuboCop's
/// `inside_class_with_stateful_parent?` only checks the nearest block ancestor.
/// If the nearest block is not `Class.new(Parent)`, no offense. Fixed by
/// tracking non-Class.new blocks as `ClassContext::Block` and stopping the
/// search when one is encountered.
///
/// FN=43: `is_inside_class_or_sclass()` returned false for modules, but
/// RuboCop's `callback_method_def?` uses `each_ancestor(:class, :sclass, :module)`
/// — callbacks inside modules should also be flagged. Fixed by including
/// `Module` in the accepted contexts for callback methods.
///
/// ## Corpus investigation round 2 (2026-03-11)
///
/// Corpus oracle reported FP=64, FN=21.
///
/// FP fixes (three root causes):
/// 1. `def self.initialize` was flagged as constructor — RuboCop's `on_defs`
///    only checks callbacks, not `initialize`. Fixed by checking
///    `node.receiver().is_none()` before the constructor check.
/// 2. `LambdaNode` was not tracked as a block context — RuboCop's
///    `:any_block` includes lambdas (`-> {}` and `lambda {}`). Added
///    `visit_lambda_node` that pushes `ClassContext::Block`.
/// 3. `SuperFinder` stopped at nested defs/classes/modules, but RuboCop's
///    `each_descendant(:super, :zsuper)` traverses into everything. A `super`
///    in a nested scope inside `def initialize` counts as "containing super"
///    for RuboCop, preventing the offense. Removed the early-return overrides
///    in `SuperFinder` to match.
///
/// ## Corpus investigation round 3 (2026-03-14)
///
/// Corpus oracle reported FP=63, FN=21.
///
/// FP fixes (two root causes):
/// 1. `is_inside_class_with_stateful_parent()` walked the class_stack from
///    innermost outward, stopping at the first class OR block. But RuboCop's
///    `inside_class_with_stateful_parent?` uses two SEPARATE ancestor searches:
///    first `each_ancestor(:any_block).first` (looks past classes), then
///    `each_ancestor(:class).first` (only if no block found). This means a
///    block wrapping a class suppresses the offense even when the class has a
///    parent (e.g., `describe do; class Foo < Bar; def initialize; end; end; end`).
///    Fixed by splitting into two-phase search: Phase 1 scans entire stack for
///    blocks (skipping classes), Phase 2 scans for classes only if no block found.
/// 2. `is_inside_class_module_or_sclass()` treated `ClassNewWithParent` and
///    `ClassNewWithoutParent` as class ancestors, but in RuboCop AST these are
///    `:block` nodes, not `:class`. `callback_method_def?` uses
///    `each_ancestor(:class, :sclass, :module)` which does NOT find blocks.
///    Fixed by making ClassNew* contexts transparent (continue) in the callback
///    ancestor check.
pub struct MissingSuper;

/// Lifecycle callback method names that require `super`.
const CLASS_LIFECYCLE_CALLBACKS: &[&[u8]] = &[b"inherited"];
const METHOD_LIFECYCLE_CALLBACKS: &[&[u8]] = &[
    b"method_added",
    b"method_removed",
    b"method_undefined",
    b"singleton_method_added",
    b"singleton_method_removed",
    b"singleton_method_undefined",
];

/// Stateless parent classes that don't need super in initialize.
const STATELESS_CLASSES: &[&[u8]] = &[b"BasicObject", b"Object"];

impl Cop for MissingSuper {
    fn name(&self) -> &'static str {
        "Lint/MissingSuper"
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
        let allowed_parent_classes: Vec<Vec<u8>> = config
            .get_string_array("AllowedParentClasses")
            .unwrap_or_default()
            .iter()
            .map(|s| s.as_bytes().to_vec())
            .collect();

        let mut visitor = MissingSuperVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            class_stack: Vec::new(),
            allowed_parent_classes,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Tracks the class context for determining parent classes.
#[derive(Debug, Clone)]
enum ClassContext {
    /// Inside a `class Foo < Parent`, with the parent class name.
    ClassWithParent(Vec<u8>),
    /// Inside a `class Foo` without parent.
    ClassWithoutParent,
    /// Inside a `Class.new(Parent) do ... end` block.
    ClassNewWithParent(Vec<u8>),
    /// Inside a `Class.new do ... end` block (no parent).
    ClassNewWithoutParent,
    /// Inside a module.
    Module,
    /// Inside `class << self`.
    Sclass,
    /// Inside a non-Class.new block (e.g., `items.each do ... end`).
    /// RuboCop checks the nearest block ancestor first — if it's not
    /// Class.new(Parent), `initialize` without super is not an offense.
    Block,
}

struct MissingSuperVisitor<'a, 'src> {
    cop: &'a MissingSuper,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    class_stack: Vec<ClassContext>,
    allowed_parent_classes: Vec<Vec<u8>>,
}

impl MissingSuperVisitor<'_, '_> {
    fn is_stateless_or_allowed(&self, parent_name: &[u8]) -> bool {
        // Extract the last segment of the constant path for comparison
        let last_segment = if let Some(pos) = parent_name.iter().rposition(|&b| b == b':') {
            &parent_name[pos + 1..]
        } else {
            parent_name
        };

        if STATELESS_CLASSES.contains(&last_segment) {
            return true;
        }
        if self
            .allowed_parent_classes
            .iter()
            .any(|s| s.as_slice() == last_segment)
        {
            return true;
        }
        false
    }

    fn is_inside_class_with_stateful_parent(&self) -> bool {
        // RuboCop's two-phase ancestor logic:
        //
        // Phase 1: Find the nearest block ancestor (each_ancestor(:any_block).first).
        //   This looks through ALL ancestors — classes, modules, sclass are all
        //   transparent during this search. If ANY block ancestor exists:
        //   - If it's Class.new(Parent), check the parent.
        //   - Otherwise (non-Class.new block), return false — no offense.
        //
        // Phase 2: Only if NO block ancestor exists at all, find the nearest
        //   class ancestor (each_ancestor(:class).first). If found with a
        //   stateful parent, offense.
        //
        // Key insight: RuboCop checks blocks and classes independently via
        // separate each_ancestor calls. A block that wraps a class still takes
        // precedence — even if the class is closer to the def than the block.

        // Phase 1: scan entire stack for nearest block
        for ctx in self.class_stack.iter().rev() {
            match ctx {
                ClassContext::ClassNewWithParent(parent) => {
                    return !self.is_stateless_or_allowed(parent);
                }
                ClassContext::ClassNewWithoutParent | ClassContext::Block => {
                    return false;
                }
                // Classes, modules, sclass are transparent during block search
                ClassContext::ClassWithParent(_)
                | ClassContext::ClassWithoutParent
                | ClassContext::Sclass
                | ClassContext::Module => continue,
            }
        }

        // Phase 2: no block ancestor found — check for class ancestor
        for ctx in self.class_stack.iter().rev() {
            match ctx {
                ClassContext::ClassWithParent(parent) => {
                    return !self.is_stateless_or_allowed(parent);
                }
                ClassContext::ClassWithoutParent => {
                    return false;
                }
                // Sclass and Module are transparent during class search
                // (RuboCop's each_ancestor(:class) skips :sclass and :module)
                ClassContext::Sclass | ClassContext::Module => continue,
                // Block/ClassNew* already handled in phase 1
                ClassContext::ClassNewWithParent(_)
                | ClassContext::ClassNewWithoutParent
                | ClassContext::Block => continue,
            }
        }
        false
    }

    fn is_inside_class_module_or_sclass(&self) -> bool {
        // RuboCop's callback_method_def? checks each_ancestor(:class, :sclass, :module).
        // Note: Class.new blocks are `:block` type in RuboCop AST, NOT `:class`,
        // so they are NOT found by this ancestor search. Only explicit `class`,
        // `class << self`, and `module` nodes count.
        for ctx in self.class_stack.iter().rev() {
            match ctx {
                ClassContext::ClassWithParent(_)
                | ClassContext::ClassWithoutParent
                | ClassContext::Sclass
                | ClassContext::Module => return true,
                // Blocks (including Class.new blocks) are transparent for
                // callback class ancestor lookup — RuboCop's
                // each_ancestor(:class, :sclass, :module) skips blocks.
                ClassContext::Block
                | ClassContext::ClassNewWithParent(_)
                | ClassContext::ClassNewWithoutParent => {
                    continue;
                }
            }
        }
        false
    }

    fn def_contains_super(node: &ruby_prism::DefNode<'_>) -> bool {
        let mut finder = SuperFinder { found: false };
        if let Some(body) = node.body() {
            finder.visit(&body);
        }
        finder.found
    }

    fn is_callback_name(name: &[u8]) -> bool {
        CLASS_LIFECYCLE_CALLBACKS.contains(&name) || METHOD_LIFECYCLE_CALLBACKS.contains(&name)
    }
}

impl<'pr> Visit<'pr> for MissingSuperVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let ctx = if let Some(superclass) = node.superclass() {
            let loc = superclass.location();
            let parent_name = self.source.as_bytes()[loc.start_offset()..loc.end_offset()].to_vec();
            ClassContext::ClassWithParent(parent_name)
        } else {
            ClassContext::ClassWithoutParent
        };
        self.class_stack.push(ctx);
        ruby_prism::visit_class_node(self, node);
        self.class_stack.pop();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        self.class_stack.push(ClassContext::Module);
        ruby_prism::visit_module_node(self, node);
        self.class_stack.pop();
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.class_stack.push(ClassContext::Sclass);
        ruby_prism::visit_singleton_class_node(self, node);
        self.class_stack.pop();
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check for Class.new(Parent) do ... end pattern
        if node.name().as_slice() == b"new" {
            if let Some(recv) = node.receiver() {
                let is_class = recv
                    .as_constant_read_node()
                    .is_some_and(|c| c.name().as_slice() == b"Class")
                    || recv
                        .as_constant_path_node()
                        .is_some_and(|cp| cp.name().is_some_and(|n| n.as_slice() == b"Class"));

                if is_class {
                    if let Some(block) = node.block() {
                        if let Some(block_node) = block.as_block_node() {
                            let ctx = if let Some(args) = node.arguments() {
                                let arg_list: Vec<_> = args.arguments().iter().collect();
                                if !arg_list.is_empty() {
                                    let first = &arg_list[0];
                                    let loc = first.location();
                                    let parent = self.source.as_bytes()
                                        [loc.start_offset()..loc.end_offset()]
                                        .to_vec();
                                    ClassContext::ClassNewWithParent(parent)
                                } else {
                                    ClassContext::ClassNewWithoutParent
                                }
                            } else {
                                ClassContext::ClassNewWithoutParent
                            };
                            self.class_stack.push(ctx);
                            ruby_prism::visit_block_node(self, &block_node);
                            self.class_stack.pop();
                            return;
                        }
                    }
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Track non-Class.new blocks. Class.new blocks are handled in
        // visit_call_node which intercepts before reaching here.
        self.class_stack.push(ClassContext::Block);
        ruby_prism::visit_block_node(self, node);
        self.class_stack.pop();
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        // Lambda blocks (-> { } and lambda { }) count as block ancestors in
        // RuboCop's :any_block. Push Block context so `initialize` inside a
        // lambda is not flagged (FP fix).
        self.class_stack.push(ClassContext::Block);
        ruby_prism::visit_lambda_node(self, node);
        self.class_stack.pop();
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let method_name = node.name().as_slice();
        let has_receiver = node.receiver().is_some();

        if method_name == b"initialize" && !has_receiver {
            // Only instance `def initialize` is a constructor.
            // `def self.initialize` is a class method — RuboCop's on_defs does
            // not check initialize, only callbacks.
            if self.is_inside_class_with_stateful_parent() && !Self::def_contains_super(node) {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Call `super` to initialize state of the parent class.".to_string(),
                ));
            }
        } else if Self::is_callback_name(method_name) {
            // Both instance and class-method callbacks (def method_added,
            // def self.inherited) need super — RuboCop handles both on_def
            // and on_defs for callbacks.
            if self.is_inside_class_module_or_sclass() && !Self::def_contains_super(node) {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Call `super` to invoke callback defined in the parent class.".to_string(),
                ));
            }
        }

        // Recurse into the body to find nested defs
        ruby_prism::visit_def_node(self, node);
    }
}

/// Finder that checks if a node tree contains a `super` or `zsuper` call.
///
/// RuboCop uses `node.each_descendant(:super, :zsuper).any?` which traverses
/// into ALL child nodes including nested defs, classes, and modules. We match
/// this behavior to avoid FPs where `super` appears inside a nested scope
/// within `def initialize`.
struct SuperFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for SuperFinder {
    fn visit_super_node(&mut self, _node: &ruby_prism::SuperNode<'pr>) {
        self.found = true;
    }

    fn visit_forwarding_super_node(&mut self, _node: &ruby_prism::ForwardingSuperNode<'pr>) {
        self.found = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MissingSuper, "cops/lint/missing_super");
}
