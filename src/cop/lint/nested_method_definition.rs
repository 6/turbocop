use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct NestedMethodDefinition;

struct NestedDefFinder {
    found: Vec<usize>,
    skip_depth: usize,
    // Stack of booleans: true if the branch node was a scope-creating node
    scope_stack: Vec<bool>,
}

impl<'pr> Visit<'pr> for NestedDefFinder {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        let is_scope = node.as_class_node().is_some()
            || node.as_module_node().is_some()
            || node.as_singleton_class_node().is_some();
        self.scope_stack.push(is_scope);
        if is_scope {
            self.skip_depth += 1;
        }
        if self.skip_depth == 0 && node.as_def_node().is_some() {
            self.found.push(node.location().start_offset());
        }
    }

    fn visit_branch_node_leave(&mut self) {
        if let Some(true) = self.scope_stack.pop() {
            self.skip_depth -= 1;
        }
    }
}

impl Cop for NestedMethodDefinition {
    fn name(&self) -> &'static str {
        "Lint/NestedMethodDefinition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let mut finder = NestedDefFinder {
            found: vec![],
            skip_depth: 0,
            scope_stack: vec![],
        };
        finder.visit(&body);

        finder
            .found
            .iter()
            .map(|&offset| {
                let (line, column) = source.offset_to_line_col(offset);
                self.diagnostic(
                    source,
                    line,
                    column,
                    "Method definitions must not be nested. Use `lambda` instead.".to_string(),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NestedMethodDefinition, "cops/lint/nested_method_definition");
}
