use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

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
        #[allow(clippy::never_loop)] // intentional: find-first via early return
        for ctx in self.class_stack.iter().rev() {
            match ctx {
                ClassContext::ClassWithParent(parent) => {
                    return !self.is_stateless_or_allowed(parent);
                }
                ClassContext::ClassNewWithParent(parent) => {
                    return !self.is_stateless_or_allowed(parent);
                }
                ClassContext::ClassWithoutParent | ClassContext::ClassNewWithoutParent => {
                    return false;
                }
                ClassContext::Sclass => {
                    // class << self is transparent, continue looking up
                    continue;
                }
                ClassContext::Module => {
                    return false;
                }
            }
        }
        false
    }

    fn is_inside_class_or_sclass(&self) -> bool {
        #[allow(clippy::never_loop)] // intentional: find-first via early return
        for ctx in self.class_stack.iter().rev() {
            match ctx {
                ClassContext::ClassWithParent(_)
                | ClassContext::ClassWithoutParent
                | ClassContext::ClassNewWithParent(_)
                | ClassContext::ClassNewWithoutParent
                | ClassContext::Sclass => return true,
                ClassContext::Module => return false,
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

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let method_name = node.name().as_slice();

        if method_name == b"initialize" {
            // Check if inside a class with a stateful parent
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
            // Instance method callbacks (like method_added) - need to be inside a class
            if self.is_inside_class_or_sclass() && !Self::def_contains_super(node) {
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

    // Don't recurse into nested defs/classes/modules
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MissingSuper, "cops/lint/missing_super");
}
