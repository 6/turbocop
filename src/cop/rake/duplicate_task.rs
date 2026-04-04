use std::collections::HashMap;

use ruby_prism::Visit;

use crate::cop::rake::RAKE_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for duplicate Rake task definitions.
///
/// When multiple tasks are defined with the same name, Rake executes all
/// of them sequentially. This is usually unintentional and confusing.
///
/// RuboCop skips tasks nested under namespaces whose names cannot be resolved
/// statically, such as `namespace adapter do`; those tasks should not be
/// flattened into the top-level namespace.
pub struct DuplicateTask;

impl Cop for DuplicateTask {
    fn name(&self) -> &'static str {
        "Rake/DuplicateTask"
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
        let mut visitor = DuplicateTaskVisitor {
            cop: self,
            source,
            namespace_stack: Vec::new(),
            tasks: HashMap::new(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct TaskInfo {
    line: usize,
}

struct DuplicateTaskVisitor<'a> {
    cop: &'a DuplicateTask,
    source: &'a SourceFile,
    namespace_stack: Vec<Option<String>>,
    tasks: HashMap<String, TaskInfo>,
    diagnostics: Vec<Diagnostic>,
}

impl DuplicateTaskVisitor<'_> {
    fn full_task_name(&self, task_name: &str) -> Option<String> {
        let namespaces = self
            .namespace_stack
            .iter()
            .map(|name| name.as_deref())
            .collect::<Option<Vec<_>>>()?;

        if namespaces.is_empty() {
            Some(task_name.to_string())
        } else {
            Some(format!("{}:{}", namespaces.join(":"), task_name))
        }
    }
}

impl<'pr> Visit<'pr> for DuplicateTaskVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let name = node.name().as_slice();

        if name == b"namespace" && node.block().is_some() && node.receiver().is_none() {
            self.namespace_stack
                .push(crate::cop::rake::extract_task_name(node));
            ruby_prism::visit_call_node(self, node);
            self.namespace_stack.pop();
            return;
        }

        if name == b"task" && node.receiver().is_none() {
            if let Some(task_name) = crate::cop::rake::extract_task_name(node) {
                let Some(full_name) = self.full_task_name(&task_name) else {
                    ruby_prism::visit_call_node(self, node);
                    return;
                };
                let loc = node.message_loc().unwrap_or(node.location());
                let (line, _) = self.source.offset_to_line_col(loc.start_offset());

                if let Some(prev) = self.tasks.get(&full_name) {
                    let (dup_line, dup_column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        dup_line,
                        dup_column,
                        format!(
                            "Task `{}` is defined at both {} (line {}) and {} (line {}).",
                            full_name,
                            self.source.path_str(),
                            prev.line,
                            self.source.path_str(),
                            dup_line,
                        ),
                    ));
                } else {
                    self.tasks.insert(full_name, TaskInfo { line });
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicateTask, "cops/rake/duplicate_task");
}
