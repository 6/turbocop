use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::{
    self, RSPEC_DEFAULT_INCLUDE, is_blank_line, is_rspec_example_group, is_rspec_let, line_at,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyLineAfterFinalLet;

impl Cop for EmptyLineAfterFinalLet {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterFinalLet"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check for example group calls (including ::RSpec.describe)
        let is_example_group = if let Some(recv) = call.receiver() {
            util::constant_name(&recv).is_some_and(|n| n == b"RSpec") && method_name == b"describe"
        } else {
            is_rspec_example_group(method_name)
        };

        if !is_example_group {
            return;
        }

        let block = match call.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return,
            },
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        // Find the last let/let! in this block
        let nodes: Vec<_> = stmts.body().iter().collect();
        let mut last_let_idx = None;
        for (i, stmt) in nodes.iter().enumerate() {
            if let Some(c) = stmt.as_call_node() {
                if c.receiver().is_none() && is_rspec_let(c.name().as_slice()) {
                    last_let_idx = Some(i);
                }
            }
        }

        let last_idx = match last_let_idx {
            Some(i) => i,
            None => return,
        };

        // Check if there's a next statement after the last let
        if last_idx + 1 >= nodes.len() {
            return; // let is the last statement
        }

        // Get the start line of the next sibling after the last let
        let next_sibling = &nodes[last_idx + 1];
        let next_start_line = {
            let loc = next_sibling.location();
            let (line, _) = source.offset_to_line_col(loc.start_offset());
            line
        };

        // Get the end line of the last let (report location)
        let last_let = &nodes[last_idx];
        let loc = last_let.location();
        let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_offset);

        // Check if there's a blank line between the last let and the next sibling.
        // We check ALL lines between the let's end line and the next sibling's start
        // line to handle cases where comments appear between them.
        let check_start = end_line + 1;
        let check_end = next_start_line;
        if check_start < check_end {
            // There are lines between the let end and next sibling start.
            // Check if ANY of those lines is blank.
            for line_num in check_start..check_end {
                if let Some(line) = line_at(source, line_num) {
                    if is_blank_line(line) {
                        return;
                    }
                }
            }
        } else {
            // Adjacent line — check the line right after the let
            let next_line = end_line + 1;
            if let Some(line) = line_at(source, next_line) {
                if is_blank_line(line) {
                    return;
                }
            } else {
                return;
            }
        }

        let let_name = if let Some(c) = last_let.as_call_node() {
            std::str::from_utf8(c.name().as_slice()).unwrap_or("let")
        } else {
            "let"
        };

        let report_col = if let Some(line_bytes) = line_at(source, end_line) {
            line_bytes.iter().take_while(|&&b| b == b' ').count()
        } else {
            0
        };

        diagnostics.push(self.diagnostic(
            source,
            end_line,
            report_col,
            format!("Add an empty line after the last `{let_name}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        EmptyLineAfterFinalLet,
        "cops/rspec/empty_line_after_final_let"
    );
}
