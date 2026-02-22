use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct MethodObjectAsBlock;

impl Cop for MethodObjectAsBlock {
    fn name(&self) -> &'static str {
        "Performance/MethodObjectAsBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = MethodObjectVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MethodObjectVisitor<'a, 'src> {
    cop: &'a MethodObjectAsBlock,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

/// Check if a block argument node contains a call to `method(...)`.
fn is_method_object_block_arg(block_arg: &ruby_prism::BlockArgumentNode<'_>) -> bool {
    let expr = match block_arg.expression() {
        Some(e) => e,
        None => return false,
    };
    let call = match expr.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    call.name().as_slice() == b"method"
}

impl<'pr> Visit<'pr> for MethodObjectVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Only flag &method(...) when parent is a send (CallNode), not super.
        // Check if this call node has a block argument that's &method(...)
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                if let Some(block_arg) = arg.as_block_argument_node() {
                    if is_method_object_block_arg(&block_arg) {
                        let loc = block_arg.location();
                        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                        self.diagnostics.push(
                            self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Use a block instead of `&method(...)` for better performance."
                                    .to_string(),
                            ),
                        );
                    }
                }
            }
        }
        // Also check the block argument slot (outside of arguments list)
        if let Some(block) = node.block() {
            if let Some(block_arg) = block.as_block_argument_node() {
                if is_method_object_block_arg(&block_arg) {
                    let loc = block_arg.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use a block instead of `&method(...)` for better performance.".to_string(),
                    ));
                }
            }
        }
        ruby_prism::visit_call_node(self, node);
    }

    // Intentionally do NOT visit super nodes — RuboCop's pattern requires ^send parent
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MethodObjectAsBlock,
        "cops/performance/method_object_as_block"
    );
}
