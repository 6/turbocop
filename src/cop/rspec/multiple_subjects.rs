use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/MultipleSubjects: Flag multiple `subject` declarations in the same example group.
///
/// This cop recursively searches within if/case branches to find all subject
/// declarations in an example group, not just direct statements. This handles
/// cases like:
///
///   describe Foo do
///     if condition
///       subject { ... }
///     else
///       subject { ... }
///     end
///   end
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

        // Collect subject declarations in this group's body, recursively searching
        // into if/case branches
        let mut subject_calls: Vec<(usize, usize)> = Vec::new(); // (line, col)

        for stmt in stmts.body().iter() {
            find_subjects_in_node(source, &stmt, &mut subject_calls);
        }

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

/// Recursively find subject declarations in a node, recursing into if/case branches
/// but stopping at nested example groups.
fn find_subjects_in_node(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    subject_calls: &mut Vec<(usize, usize)>,
) {
    // Check if this node is a subject call
    if let Some(c) = node.as_call_node() {
        let m = c.name().as_slice();
        if (m == b"subject" || m == b"subject!") && c.receiver().is_none() {
            let loc = c.location();
            let (line, col) = source.offset_to_line_col(loc.start_offset());
            subject_calls.push((line, col));
        }
        // Don't recurse into blocks attached to calls - we've already handled the call itself
        return;
    }

    // Recurse into if/elsif/else branches
    if let Some(if_node) = node.as_if_node() {
        if let Some(stmts) = if_node.statements() {
            for stmt in stmts.body().iter() {
                find_subjects_in_node(source, &stmt, subject_calls);
            }
        }
        if let Some(subsequent) = if_node.subsequent() {
            find_subjects_in_node(source, &subsequent, subject_calls);
        }
        return;
    }

    // Recurse into case/when branches
    if let Some(case_node) = node.as_case_node() {
        for cond in case_node.conditions().iter() {
            if let Some(when_node) = cond.as_when_node() {
                if let Some(stmts) = when_node.statements() {
                    for stmt in stmts.body().iter() {
                        find_subjects_in_node(source, &stmt, subject_calls);
                    }
                }
            }
        }
        if let Some(else_clause) = case_node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for stmt in stmts.body().iter() {
                    find_subjects_in_node(source, &stmt, subject_calls);
                }
            }
        }
        return;
    }

    // For other node types, try to find statements to recurse into
    if let Some(stmts) = node.as_statements_node() {
        for stmt in stmts.body().iter() {
            find_subjects_in_node(source, &stmt, subject_calls);
        }
    }
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
