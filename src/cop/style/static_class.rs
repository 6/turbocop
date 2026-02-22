use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct StaticClass;

impl Cop for StaticClass {
    fn name(&self) -> &'static str {
        "Style/StaticClass"
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
        let mut visitor = StaticClassVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct StaticClassVisitor<'a> {
    cop: &'a StaticClass,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for StaticClassVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        // Check if class has only class-level methods (def self.xxx)
        if let Some(body) = node.body() {
            if let Some(stmts) = body.as_statements_node() {
                let body_nodes: Vec<_> = stmts.body().iter().collect();
                if !body_nodes.is_empty() && all_class_methods(&body_nodes) {
                    let (line, column) = self
                        .source
                        .offset_to_line_col(node.location().start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Prefer modules to classes with only class methods.".to_string(),
                    ));
                }
            }
        }

        ruby_prism::visit_class_node(self, node);
    }
}

fn all_class_methods(nodes: &[ruby_prism::Node<'_>]) -> bool {
    let mut has_def = false;
    for node in nodes {
        if let Some(def) = node.as_def_node() {
            if def.receiver().is_some() {
                // def self.foo — class method
                has_def = true;
            } else {
                // Instance method — not a static class
                return false;
            }
        } else if node.as_singleton_class_node().is_some() {
            has_def = true;
        } else {
            // Other node types (constants, includes, etc.) — not purely static
            return false;
        }
    }
    has_def
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StaticClass, "cops/style/static_class");
}
