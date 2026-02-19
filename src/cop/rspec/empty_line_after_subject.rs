use crate::cop::util::{
    self, is_blank_line, is_rspec_example_group, is_rspec_subject, line_at, RSPEC_DEFAULT_INCLUDE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};

pub struct EmptyLineAfterSubject;

impl Cop for EmptyLineAfterSubject {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterSubject"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check for example group calls (including ::RSpec.describe)
        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).map_or(false, |n| n == b"RSpec") && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let body = match block.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let nodes: Vec<_> = stmts.body().iter().collect();
        let mut diagnostics = Vec::new();

        for (i, stmt) in nodes.iter().enumerate() {
            let c = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            let name = c.name().as_slice();
            if c.receiver().is_some() || !is_rspec_subject(name) {
                continue;
            }

            // Check if there's a next statement
            if i + 1 >= nodes.len() {
                continue; // last statement
            }

            let loc = stmt.location();
            let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(end_offset);

            // Check if next line is blank
            let next_line = end_line + 1;
            if let Some(line) = line_at(source, next_line) {
                if is_blank_line(line) {
                    continue;
                }
            } else {
                continue;
            }

            let subject_name = std::str::from_utf8(name).unwrap_or("subject");
            let (_, start_col) = source.offset_to_line_col(loc.start_offset());

            diagnostics.push(self.diagnostic(
                source,
                end_line,
                start_col,
                format!("Add an empty line after `{subject_name}`."),
            ));
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLineAfterSubject, "cops/rspec/empty_line_after_subject");
}
