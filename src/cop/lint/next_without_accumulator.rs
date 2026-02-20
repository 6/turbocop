use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct NextWithoutAccumulator;

impl Cop for NextWithoutAccumulator {
    fn name(&self) -> &'static str {
        "Lint/NextWithoutAccumulator"
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
        let mut visitor = NextWithoutAccVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_reduce_block: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct NextWithoutAccVisitor<'a, 'src> {
    cop: &'a NextWithoutAccumulator,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_reduce_block: bool,
}

impl<'pr> Visit<'pr> for NextWithoutAccVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();
        let is_reduce = method_name == b"reduce" || method_name == b"inject";

        if is_reduce && node.receiver().is_some() {
            // Check if this call has a block
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    let old = self.in_reduce_block;
                    self.in_reduce_block = true;
                    if let Some(body) = block_node.body() {
                        self.visit(&body);
                    }
                    self.in_reduce_block = old;
                    return;
                }
            }
        }

        // Visit receiver and arguments normally
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            self.visit(&args.as_node());
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }

    fn visit_next_node(&mut self, node: &ruby_prism::NextNode<'pr>) {
        if self.in_reduce_block && node.arguments().is_none() {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Use `next` with an accumulator argument in a `reduce`.".to_string(),
            ));
        }
    }

    // Don't recurse into nested methods/classes
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NextWithoutAccumulator, "cops/lint/next_without_accumulator");
}
