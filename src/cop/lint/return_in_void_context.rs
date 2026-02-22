use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct ReturnInVoidContext;

impl Cop for ReturnInVoidContext {
    fn name(&self) -> &'static str {
        "Lint/ReturnInVoidContext"
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
        let mut visitor = InitializeVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct InitializeVisitor<'a, 'src> {
    cop: &'a ReturnInVoidContext,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for InitializeVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if node.name().as_slice() != b"initialize" {
            // Still recurse into nested defs (though unlikely)
            ruby_prism::visit_def_node(self, node);
            return;
        }

        // Found initialize method, look for return nodes with values
        let mut finder = ReturnWithValueFinder {
            offsets: Vec::new(),
        };
        if let Some(body) = node.body() {
            finder.visit(&body);
        }

        for offset in finder.offsets {
            let (line, column) = self.source.offset_to_line_col(offset);
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not return a value in `initialize`.".to_string(),
            ));
        }
    }
}

struct ReturnWithValueFinder {
    offsets: Vec<usize>,
}

impl<'pr> Visit<'pr> for ReturnWithValueFinder {
    fn visit_return_node(&mut self, node: &ruby_prism::ReturnNode<'pr>) {
        // ReturnNode with arguments means `return value`
        if node.arguments().is_some() {
            self.offsets.push(node.location().start_offset());
        }
        ruby_prism::visit_return_node(self, node);
    }

    // Don't recurse into nested def/class/module
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReturnInVoidContext, "cops/lint/return_in_void_context");
}
