use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};

/// RSpec/MultipleSubjects: Flag multiple `subject` declarations in the same example group.
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
    ) -> Vec<Diagnostic> {
        // Look for call nodes that are example groups (describe/context/etc.)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();
        if !is_example_group(name) {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };
        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Collect subject declarations in this group's direct body
        let mut subject_calls: Vec<(usize, usize, usize)> = Vec::new(); // (line, col, end_offset)

        for stmt in stmts.body().iter() {
            if let Some(c) = stmt.as_call_node() {
                let m = c.name().as_slice();
                if (m == b"subject" || m == b"subject!") && c.receiver().is_none() {
                    let loc = c.location();
                    let (line, col) = source.offset_to_line_col(loc.start_offset());
                    let end_off = loc.end_offset();
                    subject_calls.push((line, col, end_off));
                }
            }
        }

        if subject_calls.len() <= 1 {
            return Vec::new();
        }

        // Flag all except the last one
        let mut diagnostics = Vec::new();
        for &(line, col, _end_off) in &subject_calls[..subject_calls.len() - 1] {
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Do not set more than one subject per example group".to_string(),
            ));
        }

        diagnostics
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
