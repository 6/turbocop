use ruby_prism::Visit;

use crate::cop::rake::RAKE_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for method definitions inside Rake task or namespace blocks.
///
/// Methods defined inside task/namespace blocks are actually created at the
/// top level despite their syntactic location, which is misleading.
pub struct MethodDefinitionInTask;

impl Cop for MethodDefinitionInTask {
    fn name(&self) -> &'static str {
        "Rake/MethodDefinitionInTask"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        RAKE_DEFAULT_INCLUDE
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
        let mut visitor = MethodInTaskVisitor {
            cop: self,
            source,
            in_task_or_namespace: false,
            in_class_definition: false,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct MethodInTaskVisitor<'a> {
    cop: &'a MethodDefinitionInTask,
    source: &'a SourceFile,
    in_task_or_namespace: bool,
    in_class_definition: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for MethodInTaskVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();
        let is_task_or_ns = (name == b"task" || name == b"namespace") && node.block().is_some();

        if is_task_or_ns {
            let was = self.in_task_or_namespace;
            self.in_task_or_namespace = true;
            ruby_prism::visit_call_node(self, node);
            self.in_task_or_namespace = was;
        } else {
            ruby_prism::visit_call_node(self, node);
        }
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let was = self.in_class_definition;
        self.in_class_definition = true;
        ruby_prism::visit_class_node(self, node);
        self.in_class_definition = was;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let was = self.in_class_definition;
        self.in_class_definition = true;
        ruby_prism::visit_module_node(self, node);
        self.in_class_definition = was;
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        let was = self.in_class_definition;
        self.in_class_definition = true;
        ruby_prism::visit_singleton_class_node(self, node);
        self.in_class_definition = was;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if self.in_task_or_namespace && !self.in_class_definition {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not define a method in rake task, because it will be defined to the top level."
                    .to_string(),
            ));
        }
        ruby_prism::visit_def_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MethodDefinitionInTask,
        "cops/rake/method_definition_in_task"
    );
}
