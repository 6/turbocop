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

/// Check if a block argument node contains a call to `method(:symbol)`.
/// RuboCop only flags `&method(:sym)` where the argument is a symbol literal,
/// not `&method(variable)` or `&method("string")`.
fn is_method_object_block_arg(block_arg: &ruby_prism::BlockArgumentNode<'_>) -> bool {
    let expr = match block_arg.expression() {
        Some(e) => e,
        None => return false,
    };
    let call = match expr.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    if call.name().as_slice() != b"method" {
        return false;
    }
    // Require exactly one argument that is a symbol literal
    let args = match call.arguments() {
        Some(a) => a,
        None => return false,
    };
    let arg_list = args.arguments();
    arg_list.len() == 1 && arg_list.iter().next().unwrap().as_symbol_node().is_some()
}

impl<'pr> Visit<'pr> for MethodObjectVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Only flag &method(...) when parent is a send (CallNode), not super.
        // RuboCop's pattern uses ^send which excludes csend (safe navigation &.),
        // so skip when the parent call uses safe navigation.
        let is_safe_nav = if let Some(op) = node.call_operator_loc() {
            op.as_slice() == b"&."
        } else {
            false
        };
        if !is_safe_nav {
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
