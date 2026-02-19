use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantConstantBase;

impl Cop for RedundantConstantBase {
    fn name(&self) -> &'static str {
        "Style/RedundantConstantBase"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut visitor = RedundantConstantBaseVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            in_class_or_module: false,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct RedundantConstantBaseVisitor<'a> {
    cop: &'a RedundantConstantBase,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    in_class_or_module: bool,
}

impl<'pr> Visit<'pr> for RedundantConstantBaseVisitor<'_> {
    fn visit_constant_path_node(&mut self, node: &ruby_prism::ConstantPathNode<'pr>) {
        // Check for ::Foo at the top level (parent is None = cbase, and not inside class/module)
        if node.parent().is_none() && !self.in_class_or_module {
            // This is a ::Foo reference at top level - redundant ::
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Remove redundant `::`.".to_string(),
            ));
        }

        // Visit children
        if let Some(parent) = node.parent() {
            self.visit(&parent);
        }
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let prev = self.in_class_or_module;
        self.in_class_or_module = true;
        // Visit constant path but not as redundant
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_class_or_module = prev;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let prev = self.in_class_or_module;
        self.in_class_or_module = true;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.in_class_or_module = prev;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        // sclass is NOT a new constant scope - treat as top level
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantConstantBase, "cops/style/redundant_constant_base");
}
