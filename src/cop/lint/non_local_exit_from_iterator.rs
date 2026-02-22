use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct NonLocalExitFromIterator;

impl Cop for NonLocalExitFromIterator {
    fn name(&self) -> &'static str {
        "Lint/NonLocalExitFromIterator"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let mut visitor = NonLocalExitVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            block_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Tracks block context for determining whether a `return` is a non-local exit.
#[derive(Clone)]
struct BlockContext {
    has_args: bool,
    is_chained_send: bool,
    is_define_method: bool,
}

struct NonLocalExitVisitor<'a, 'src> {
    cop: &'a NonLocalExitFromIterator,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    block_stack: Vec<BlockContext>,
}

impl<'pr> Visit<'pr> for NonLocalExitVisitor<'_, '_> {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        // Per RuboCop: only flag `return` with NO return value
        if node.arguments().is_some() {
            return;
        }

        if let Some(ctx) = self.block_stack.last() {
            if ctx.is_define_method {
                return; // define_method creates its own scope for return
            }
            if !ctx.has_args {
                return; // Block without arguments
            }
            if !ctx.is_chained_send {
                return; // Block without chained method
            }
            // This is a non-local exit from an iterator
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Non-local exit from iterator detected. Use `next` or `break` instead of `return`."
                    .to_string(),
            ));
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let has_args = node.parameters().is_some();
        // Default: standalone block (no call context known)
        self.block_stack.push(BlockContext {
            has_args,
            is_chained_send: false,
            is_define_method: false,
        });
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.block_stack.pop();
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Visit receiver first
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        // Visit arguments
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }
        // If call has a block, push block context and visit block body
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                let has_args = block_node.parameters().is_some();
                let is_chained_send = node.receiver().is_some();
                let method_name = node.name().as_slice();
                let is_define_method =
                    method_name == b"define_method" || method_name == b"define_singleton_method";

                self.block_stack.push(BlockContext {
                    has_args,
                    is_chained_send,
                    is_define_method,
                });
                if let Some(body) = block_node.body() {
                    self.visit(&body);
                }
                self.block_stack.pop();
            } else {
                // BlockArgumentNode (&block) - visit it normally
                self.visit(&block);
            }
        }
    }

    // Don't recurse into nested def/class/module/lambda (they create their own scope)
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NonLocalExitFromIterator,
        "cops/lint/non_local_exit_from_iterator"
    );
}
