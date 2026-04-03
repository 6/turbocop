use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Flags bare `attr` calls, including parenthesized forms like `attr(:name)`.
///
/// A prior guard skipped every call with an `opening_loc()`, which caused false
/// negatives for parenthesized `attr(...)` in class bodies, module methods, and
/// DSL-style helper methods. RuboCop still registers those offenses, so this cop
/// now relies on the existing custom-`attr` context guard instead of treating
/// parentheses as an exemption, and it uses the last boolean argument to choose
/// between `attr_reader` and `attr_accessor`.
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
        let last_arg = arg_list.last();

        // RuboCop keys off the last boolean arg, so `attr('docdir', '', true)`
        // still becomes `attr_accessor`.
        let has_true_arg = last_arg.is_some_and(|arg| arg.as_true_node().is_some());
        let has_false_arg = last_arg.is_some_and(|arg| arg.as_false_node().is_some());
        let has_boolean_last_arg = has_true_arg || has_false_arg;

        let replacement = if has_true_arg {
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
            corr.push(crate::correction::Correction {
                start: msg_loc.start_offset(),
                end: msg_loc.end_offset(),
                replacement: replacement.to_string(),
                cop_name: self.name(),
                cop_index: 0,
            });

            if has_boolean_last_arg {
                // Match RuboCop by keeping the first arg and any surrounding
                // parentheses, while deleting trailing args through the closing
                // paren when present.
                let first_arg = &arg_list[0];
                let delete_end = call_node.closing_loc().map_or_else(
                    || call_node.location().end_offset(),
                    |loc| loc.start_offset(),
                );
                corr.push(crate::correction::Correction {
                    start: first_arg.location().end_offset(),
                    end: delete_end,
                    replacement: String::new(),
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
