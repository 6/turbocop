use crate::cop::shared::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/MultipleSubjects: Flag multiple `subject` declarations in the same example group.
///
/// Investigation (2026-03-23):
/// corpus FNs came from subject declarations wrapped in top-level `if` branches
/// inside a `describe` body. The previous implementation only checked direct
/// `CallNode` statements, so it missed `subject`/`subject!` calls nested under
/// `if`/`elsif`/`else` and `unless` branches in the same example-group scope.
/// Fix: recursively walk conditional branches when collecting subject
/// declarations, while still avoiding traversal into nested block scopes.
pub struct MultipleSubjects;

impl Cop for MultipleSubjects {
    fn name(&self) -> &'static str {
        "RSpec/MultipleSubjects"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, STATEMENTS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Look for call nodes that are example groups (describe/context/etc.)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        if !is_example_group(name) {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Collect subject declarations in this group's body, including
        // top-level conditional branches (`if`/`elsif`/`else`, `unless`).
        let mut subject_calls: Vec<(usize, usize)> = Vec::new(); // (line, col)
        self.collect_subject_calls_from_statements(source, &stmts, &mut subject_calls);

        if subject_calls.len() <= 1 {
            return;
        }

        // Flag all except the last one
        for &(line, col) in &subject_calls[..subject_calls.len() - 1] {
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Do not set more than one subject per example group".to_string(),
            ));
        }
    }
}

impl MultipleSubjects {
    fn collect_subject_calls_from_statements(
        &self,
        source: &SourceFile,
        stmts: &ruby_prism::StatementsNode<'_>,
        subject_calls: &mut Vec<(usize, usize)>,
    ) {
        for stmt in stmts.body().iter() {
            self.collect_subject_calls(source, &stmt, subject_calls);
        }
    }

    fn collect_subject_calls(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        subject_calls: &mut Vec<(usize, usize)>,
    ) {
        if let Some(call) = node.as_call_node() {
            if is_subject_declaration(&call) {
                let loc = call.location();
                let (line, col) = source.offset_to_line_col(loc.start_offset());
                subject_calls.push((line, col));
            }
            return;
        }

        if let Some(if_node) = node.as_if_node() {
            if let Some(stmts) = if_node.statements() {
                self.collect_subject_calls_from_statements(source, &stmts, subject_calls);
            }
            if let Some(subsequent) = if_node.subsequent() {
                self.collect_subject_calls(source, &subsequent, subject_calls);
            }
            return;
        }

        if let Some(unless_node) = node.as_unless_node() {
            if let Some(stmts) = unless_node.statements() {
                self.collect_subject_calls_from_statements(source, &stmts, subject_calls);
            }
            if let Some(else_clause) = unless_node.else_clause() {
                if let Some(stmts) = else_clause.statements() {
                    self.collect_subject_calls_from_statements(source, &stmts, subject_calls);
                }
            }
            return;
        }

        if let Some(else_node) = node.as_else_node()
            && let Some(stmts) = else_node.statements()
        {
            self.collect_subject_calls_from_statements(source, &stmts, subject_calls);
        }
    }
}

fn is_subject_declaration(call: &ruby_prism::CallNode<'_>) -> bool {
    let name = call.name().as_slice();
    (name == b"subject" || name == b"subject!") && call.receiver().is_none()
}

fn is_example_group(name: &[u8]) -> bool {
    matches!(
        name,
        b"describe"
            | b"context"
            | b"feature"
            | b"example_group"
            | b"xdescribe"
            | b"xcontext"
            | b"xfeature"
            | b"fdescribe"
            | b"fcontext"
            | b"ffeature"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(MultipleSubjects, "cops/rspec/multiple_subjects");
}
