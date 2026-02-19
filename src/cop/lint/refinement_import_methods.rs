use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks if `include` or `prepend` is called in a `refine` block.
/// These methods are deprecated and should be replaced with `import_methods`.
pub struct RefinementImportMethods;

impl Cop for RefinementImportMethods {
    fn name(&self) -> &'static str {
        "Lint/RefinementImportMethods"
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
    ) {
        let mut visitor = RefineVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_refine_block: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RefineVisitor<'a, 'src> {
    cop: &'a RefinementImportMethods,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_refine_block: bool,
}

impl<'pr> Visit<'pr> for RefineVisitor<'_, '_> {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        // Check if this is a `refine ... do ... end` block
        // The block's call is the parent CallNode
        // In Prism, block_node wraps the call, so we need to check the call
        // Actually in Prism, block_node doesn't directly expose the method name.
        // We need to check via the source or by walking up.
        // Block nodes don't have a method name, but blocks are children of call nodes.
        // Actually in Prism, a block call like `refine Foo do ... end` creates a
        // CallNode with block = BlockNode.
        // But when visiting, we visit the CallNode first which may have .block().
        // Let me just check if this block is the block of a `refine` call.

        // Unfortunately, BlockNode doesn't have a direct reference to its call.
        // But we can check from the call side. Let me use the in_refine_block flag set from call_node.
        let old = self.in_refine_block;
        ruby_prism::visit_block_node(self, node);
        self.in_refine_block = old;
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Check if this is a `refine` call with a block
        if method_name == b"refine" && node.receiver().is_none() && node.block().is_some() {
            let old = self.in_refine_block;
            self.in_refine_block = true;
            if let Some(block) = node.block() {
                self.visit(&block);
            }
            self.in_refine_block = old;
            // Visit receiver and args but not block again
            if let Some(args) = node.arguments() {
                self.visit(&args.as_node());
            }
            return;
        }

        // Check if this is include/prepend inside a refine block
        if self.in_refine_block
            && (method_name == b"include" || method_name == b"prepend")
            && node.receiver().is_none()
        {
            let msg_loc = node.message_loc().unwrap_or(node.location());
            let (line, column) = self.source.offset_to_line_col(msg_loc.start_offset());
            let method_str = if method_name == b"include" {
                "include"
            } else {
                "prepend"
            };
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!(
                    "Use `import_methods` instead of `{}` because it is deprecated in Ruby 3.1.",
                    method_str
                ),
            ));
        }

        // Visit children
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
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RefinementImportMethods, "cops/lint/refinement_import_methods");
}
