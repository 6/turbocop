use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, KEYWORD_HASH_NODE, STATEMENTS_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IsExpectedSpecify;

impl Cop for IsExpectedSpecify {
    fn name(&self) -> &'static str {
        "RSpec/IsExpectedSpecify"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, KEYWORD_HASH_NODE, STATEMENTS_NODE]
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

        if call.receiver().is_some() {
            return;
        }

        if call.name().as_slice() != b"specify" {
            return;
        }

        // Must be a one-liner with a brace block (not do...end)
        let block_raw = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block = match block_raw.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Must have no description argument (positional args)
        if let Some(args) = call.arguments() {
            for arg in args.arguments().iter() {
                if arg.as_keyword_hash_node().is_none() {
                    return;
                }
            }
        }

        // Check if it's a single-line block
        let block_loc = block.location();
        let (start_line, _) = source.offset_to_line_col(block_loc.start_offset());
        let end_off = block_loc
            .end_offset()
            .saturating_sub(1)
            .max(block_loc.start_offset());
        let (end_line, _) = source.offset_to_line_col(end_off);

        if start_line != end_line {
            return;
        }

        // Check if the block body contains is_expected or are_expected
        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        if contains_is_expected(&body) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use `it` instead of `specify`.".to_string(),
            ));
        }
    }
}

/// Check if a node (or its descendants) contains an `is_expected` or `are_expected` call.
fn contains_is_expected(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        let name = call.name().as_slice();
        if (name == b"is_expected" || name == b"are_expected") && call.receiver().is_none() {
            return true;
        }
        // Check receiver chain
        if let Some(recv) = call.receiver() {
            if contains_is_expected(&recv) {
                return true;
            }
        }
    }

    // Check statements node
    if let Some(stmts) = node.as_statements_node() {
        for child in stmts.body().iter() {
            if contains_is_expected(&child) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IsExpectedSpecify, "cops/rspec/is_expected_specify");
}
