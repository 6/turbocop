use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DisjunctiveAssignmentInConstructor;

impl Cop for DisjunctiveAssignmentInConstructor {
    fn name(&self) -> &'static str {
        "Lint/DisjunctiveAssignmentInConstructor"
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
        let mut visitor = InitVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct InitVisitor<'a, 'src> {
    cop: &'a DisjunctiveAssignmentInConstructor,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
}

/// Returns true if the node is any kind of `||=` (disjunctive/or-write) assignment.
/// In RuboCop's Parser AST, all of these are `:or_asgn` nodes.
fn is_or_write_node(node: &ruby_prism::Node<'_>) -> bool {
    matches!(
        node,
        ruby_prism::Node::LocalVariableOrWriteNode { .. }
            | ruby_prism::Node::InstanceVariableOrWriteNode { .. }
            | ruby_prism::Node::ClassVariableOrWriteNode { .. }
            | ruby_prism::Node::GlobalVariableOrWriteNode { .. }
            | ruby_prism::Node::ConstantOrWriteNode { .. }
            | ruby_prism::Node::ConstantPathOrWriteNode { .. }
    )
}

impl<'pr> Visit<'pr> for InitVisitor<'_, '_> {
    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if node.name().as_slice() != b"initialize" {
            ruby_prism::visit_def_node(self, node);
            return;
        }

        // Check body for ||= on instance variables
        let body = match node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = if let Some(s) = body.as_statements_node() {
            s.body()
        } else {
            // Single statement body
            check_or_asgn(self, &body);
            return;
        };

        // RuboCop only flags ||= on ivars that appear at the BEGINNING of the
        // initialize body, before any non-||= statement. Once a non-||=
        // statement is encountered, it breaks — because after arbitrary code
        // runs, a disjunctive assignment may actually be necessary.
        for stmt in stmts.iter() {
            if is_or_write_node(&stmt) {
                // It's some kind of ||=; only flag instance variable ones
                if let Some(ivar_or) = stmt.as_instance_variable_or_write_node() {
                    let loc = ivar_or.operator_loc();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Unnecessary disjunctive assignment. Use plain assignment.".to_string(),
                    ));
                }
            } else {
                // Non-||= statement encountered; stop checking
                break;
            }
        }
    }
}

fn check_or_asgn(visitor: &mut InitVisitor<'_, '_>, node: &ruby_prism::Node<'_>) {
    if !is_or_write_node(node) {
        return;
    }
    if let Some(ivar_or) = node.as_instance_variable_or_write_node() {
        let loc = ivar_or.operator_loc();
        let (line, column) = visitor.source.offset_to_line_col(loc.start_offset());
        visitor.diagnostics.push(visitor.cop.diagnostic(
            visitor.source,
            line,
            column,
            "Unnecessary disjunctive assignment. Use plain assignment.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DisjunctiveAssignmentInConstructor, "cops/lint/disjunctive_assignment_in_constructor");
}
