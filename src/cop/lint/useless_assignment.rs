use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

pub struct UselessAssignment;

impl Cop for UselessAssignment {
    fn name(&self) -> &'static str {
        "Lint/UselessAssignment"
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
        let mut visitor = UselessAssignVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct UselessAssignVisitor<'a, 'src> {
    cop: &'a UselessAssignment,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

struct WriteInfo {
    name: String,
    offset: usize,
}

struct WriteReadCollector {
    writes: Vec<WriteInfo>,
    reads: HashSet<String>,
}

impl WriteReadCollector {
    fn new() -> Self {
        Self {
            writes: Vec::new(),
            reads: HashSet::new(),
        }
    }
}

impl<'pr> Visit<'pr> for WriteReadCollector {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.writes.push(WriteInfo {
            name,
            offset: node.name_loc().start_offset(),
        });
        // Visit the value (it might contain reads)
        self.visit(&node.value());
    }

    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        self.reads.insert(name);
    }

    // Don't recurse into nested scopes
    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}
    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}
    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}
}

impl<'pr> Visit<'pr> for UselessAssignVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(body) = node.body() {
            let mut collector = WriteReadCollector::new();
            collector.visit(&body);

            // Find writes that are never read
            for write in &collector.writes {
                if write.name.starts_with('_') {
                    continue;
                }
                if !collector.reads.contains(&write.name) {
                    let (line, column) = self.source.offset_to_line_col(write.offset);
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        format!("Useless assignment to variable - `{}`.", write.name),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessAssignment, "cops/lint/useless_assignment");
}
