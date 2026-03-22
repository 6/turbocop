use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/MultipleSubjects: Flag multiple `subject` declarations in the same example group.
///
/// This cop recursively searches for subject declarations within an example group,
/// including those inside conditional branches (if/elsif/else). It stops at nested
/// example groups, shared example groups (it_behaves_like), and examples (it/specify),
/// which define their own scope.
///
/// Fixes FN where subjects inside `if`/`unless` branches were not detected:
/// ```ruby
/// describe Foo do
///   if condition
///     subject { A }
///   else
///     subject { B }  # Was not detected before
///   end
/// end
/// ```
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

        // Recursively collect all subject declarations in this group's body
        let mut subject_calls: Vec<(usize, usize)> = Vec::new(); // (line, col)
        find_subjects_in_scope(&body, source, &mut subject_calls);

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

/// Recursively find subject declarations within a node, stopping at nested
/// example groups and examples (which define their own scope).
fn find_subjects_in_scope(
    node: &ruby_prism::Node<'_>,
    source: &SourceFile,
    results: &mut Vec<(usize, usize)>,
) {
    // Check if this node itself is a subject call
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"subject" || name == b"subject!") && call.receiver().is_none() {
            let loc = call.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            results.push((line, col));
            return;
        }
    }

    // Stop at nested example groups and examples (scope boundaries)
    if is_example_group_node(node) || is_example_node(node) {
        return;
    }

    // Recurse based on node type
    if let Some(if_node) = node.as_if_node() {
        // Recurse into if/unless/elsif branches
        if let Some(stmts) = if_node.statements() {
            for stmt in stmts.body().iter() {
                find_subjects_in_scope(&stmt, source, results);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            find_subjects_in_scope(&subsequent, source, results);
        }
    } else if let Some(unless_node) = node.as_unless_node() {
        if let Some(stmts) = unless_node.statements() {
            for stmt in stmts.body().iter() {
                find_subjects_in_scope(&stmt, source, results);
            }
        }
        if let Some(else_clause) = unless_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for stmt in stmts.body().iter() {
                    find_subjects_in_scope(&stmt, source, results);
                }
            }
        }
    } else if let Some(statements_node) = node.as_statements_node() {
        for stmt in statements_node.body().iter() {
            find_subjects_in_scope(&stmt, source, results);
        }
    } else if let Some(block_node) = node.as_block_node() {
        if let Some(body) = block_node.body() {
            find_subjects_in_scope(&body, source, results);
        }
    }
}

/// Check if a node is an example group (describe/context/etc. with a block).
fn is_example_group_node(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let name = call.name().as_slice();
    (is_example_group(name) || is_shared_group(name)) && call.block().is_some()
}

/// Check if a node is a shared example group (it_behaves_like).
fn is_shared_group(name: &[u8]) -> bool {
    matches!(
        name,
        b"it_behaves_like" | b"it_should_behave_like" | b"include_context"
    )
}

/// Check if a node is an example (it/specify/etc. with a block).
fn is_example_node(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };
    let name = call.name().as_slice();
    matches!(
        name,
        b"it"
            | b"specify"
            | b"example"
            | b"xexample"
            | b"fexample"
            | b"xspecify"
            | b"fspecify"
            | b"fit"
            | b"focus"
            | b"skip"
    ) && call.block().is_some()
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
