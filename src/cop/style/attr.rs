use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Matches RuboCop's bare `attr` detection for receiverless calls, including
/// parenthesized `attr(...)` sends and calls whose last boolean argument changes
/// the suggested replacement method. The previous Prism port skipped every
/// parenthesized call, which caused the remaining Style/Attr false negatives.
pub struct Attr;

impl Cop for Attr {
    fn name(&self) -> &'static str {
        "Style/Attr"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be a bare `attr` call (no receiver)
        if call_node.name().as_slice() != b"attr" {
            return;
        }
        if call_node.receiver().is_some() {
            return;
        }
        // Must have arguments
        let args = match call_node.arguments() {
            Some(a) => a,
            None => return,
        };

        if allowed_context(parse_result, call_node.location().start_offset()) {
            return;
        }

        let arg_list: Vec<_> = args.arguments().iter().collect();

        if arg_list.is_empty() {
            return;
        }

        let second_arg_is_boolean = arg_list
            .get(1)
            .is_some_and(|arg| arg.as_true_node().is_some() || arg.as_false_node().is_some());
        let replacement = if arg_list
            .last()
            .is_some_and(|arg| arg.as_true_node().is_some())
        {
            "attr_accessor"
        } else {
            "attr_reader"
        };

        let msg_loc = call_node
            .message_loc()
            .unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        let mut diag = self.diagnostic(
            source,
            line,
            column,
            format!("Do not use `attr`. Use `{replacement}` instead."),
        );
        if let Some(ref mut corr) = corrections {
            if second_arg_is_boolean {
                // Replace the entire call: `attr :name, true/false` → `attr_accessor/attr_reader :name`
                // We need to replace from `attr` through the boolean arg, keeping only the first arg
                let first_arg = &arg_list[0];
                let first_arg_str = source.byte_slice(
                    first_arg.location().start_offset(),
                    first_arg.location().end_offset(),
                    "",
                );
                corr.push(crate::correction::Correction {
                    start: msg_loc.start_offset(),
                    end: call_node.location().end_offset(),
                    replacement: format!("{replacement} {first_arg_str}"),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            } else {
                // Simple replacement: `attr` → `attr_reader`
                corr.push(crate::correction::Correction {
                    start: msg_loc.start_offset(),
                    end: msg_loc.end_offset(),
                    replacement: replacement.to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
            }
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

fn allowed_context(parse_result: &ruby_prism::ParseResult<'_>, target_offset: usize) -> bool {
    // Current strategy mirrors RuboCop's `allowed_context?`:
    // - skip `attr` in non-`class_eval`/`module_eval` blocks
    // - skip when the nearest class/block scope defines its own `def attr`
    let mut finder = AttrContextFinder {
        target_offset,
        allowed: false,
        done: false,
        scope_stack: Vec::new(),
    };
    finder.visit(&parse_result.node());
    finder.allowed
}

#[derive(Clone, Copy)]
enum ScopeKind {
    Class,
    Block { is_class_or_module_eval: bool },
}

#[derive(Clone, Copy)]
struct ScopeContext {
    kind: ScopeKind,
    defines_attr_method: bool,
}

fn scope_allows_attr_call(stack: &[ScopeContext]) -> bool {
    let Some(scope) = stack.last() else {
        return false;
    };

    match scope.kind {
        ScopeKind::Block {
            is_class_or_module_eval: false,
        } => true,
        _ => scope.defines_attr_method,
    }
}

struct AttrContextFinder {
    target_offset: usize,
    allowed: bool,
    done: bool,
    scope_stack: Vec<ScopeContext>,
}

impl<'a> Visit<'a> for AttrContextFinder {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'a>) {
        if self.done {
            return;
        }
        let defines_attr_method = scope_defines_attr_method(&node.as_node());
        self.scope_stack.push(ScopeContext {
            kind: ScopeKind::Class,
            defines_attr_method,
        });
        ruby_prism::visit_class_node(self, node);
        self.scope_stack.pop();
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'a>) {
        if self.done {
            return;
        }

        if node.location().start_offset() == self.target_offset
            && node.name().as_slice() == b"attr"
            && node.receiver().is_none()
        {
            self.allowed = scope_allows_attr_call(&self.scope_stack);
            self.done = true;
            return;
        }

        if let Some(receiver) = node.receiver() {
            self.visit(&receiver);
            if self.done {
                return;
            }
        }

        if let Some(arguments) = node.arguments() {
            self.visit(&arguments.as_node());
            if self.done {
                return;
            }
        }

        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                let method_name = node.name().as_slice();
                let is_class_or_module_eval =
                    method_name == b"class_eval" || method_name == b"module_eval";
                let defines_attr_method = scope_defines_attr_method(&block_node.as_node());
                self.scope_stack.push(ScopeContext {
                    kind: ScopeKind::Block {
                        is_class_or_module_eval,
                    },
                    defines_attr_method,
                });
                ruby_prism::visit_block_node(self, &block_node);
                self.scope_stack.pop();
            } else {
                self.visit(&block);
            }
        }
    }
}

fn scope_defines_attr_method(scope: &ruby_prism::Node<'_>) -> bool {
    let mut finder = AttrMethodFinder { found: false };
    finder.visit(scope);
    finder.found
}

struct AttrMethodFinder {
    found: bool,
}

impl<'a> Visit<'a> for AttrMethodFinder {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'a>) {
        if self.found {
            return;
        }
        if node.name().as_slice() == b"attr" {
            self.found = true;
            return;
        }
        ruby_prism::visit_def_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Attr, "cops/style/attr");
    crate::cop_autocorrect_fixture_tests!(Attr, "cops/style/attr");
}
