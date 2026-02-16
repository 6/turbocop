use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct DuplicateMethods;

impl Cop for DuplicateMethods {
    fn name(&self) -> &'static str {
        "Lint/DuplicateMethods"
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
    ) -> Vec<Diagnostic> {
        let mut visitor = DupMethodVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct DupMethodVisitor<'a, 'src> {
    cop: &'a DuplicateMethods,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

fn check_body_for_dup_methods<'src>(
    cop: &DuplicateMethods,
    source: &'src SourceFile,
    body: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let stmts = if let Some(stmts) = body.as_statements_node() {
        stmts
    } else {
        return;
    };

    let mut seen: HashMap<Vec<u8>, usize> = HashMap::new();

    for stmt in stmts.body().iter() {
        if let Some(def_node) = stmt.as_def_node() {
            let name = def_node.name().as_slice().to_vec();
            let loc = def_node.location();
            if seen.contains_key(&name) {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Duplicated method definition.".to_string(),
                ));
            } else {
                seen.insert(name, loc.start_offset());
            }
        }
    }
}

impl<'pr> Visit<'pr> for DupMethodVisitor<'_, '_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            check_body_for_dup_methods(self.cop, self.source, &body, &mut self.diagnostics);
        }
        ruby_prism::visit_class_node(self, node);
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            check_body_for_dup_methods(self.cop, self.source, &body, &mut self.diagnostics);
        }
        ruby_prism::visit_module_node(self, node);
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        if let Some(body) = node.body() {
            check_body_for_dup_methods(self.cop, self.source, &body, &mut self.diagnostics);
        }
        ruby_prism::visit_singleton_class_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateMethods, "cops/lint/duplicate_methods");
}
