use ruby_prism::Visit;

use crate::cop::rake::RAKE_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for class or module definitions inside Rake task or namespace blocks.
///
/// Such definitions are actually created at the top level despite their
/// syntactic location, which is misleading.
pub struct ClassDefinitionInTask;

impl Cop for ClassDefinitionInTask {
    fn name(&self) -> &'static str {
        "Rake/ClassDefinitionInTask"
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
        let mut visitor = TaskDefinitionVisitor {
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

struct TaskDefinitionVisitor<'a> {
    cop: &'a ClassDefinitionInTask,
    source: &'a SourceFile,
    in_task_or_namespace: bool,
    in_class_definition: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for TaskDefinitionVisitor<'_> {
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
        if self.in_task_or_namespace && !self.in_class_definition {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not define a class in rake task, because it will be defined to the top level."
                    .to_string(),
            ));
        }

        let was = self.in_class_definition;
        self.in_class_definition = true;
        ruby_prism::visit_class_node(self, node);
        self.in_class_definition = was;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if self.in_task_or_namespace && !self.in_class_definition {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Do not define a module in rake task, because it will be defined to the top level."
                    .to_string(),
            ));
        }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ClassDefinitionInTask, "cops/rake/class_definition_in_task");
}
