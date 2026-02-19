use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::BLOCK_NODE;

pub struct BlockEndNewline;

impl Cop for BlockEndNewline {
    fn name(&self) -> &'static str {
        "Layout/BlockEndNewline"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let opening_loc = block_node.opening_loc();
        let closing_loc = block_node.closing_loc();

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing_loc.start_offset());

        // Single line block — no offense
        if open_line == close_line {
            return;
        }

        // Check if `end` or `}` begins its line (only whitespace before it)
        let bytes = source.as_bytes();
        let mut pos = closing_loc.start_offset();
        while pos > 0 && bytes[pos - 1] != b'\n' {
            pos -= 1;
        }

        // Check if everything from line start to closing is whitespace
        let before_close = &bytes[pos..closing_loc.start_offset()];
        let begins_line = before_close.iter().all(|&b| b == b' ' || b == b'\t');

        if !begins_line {
            diagnostics.push(self.diagnostic(
                source,
                close_line,
                close_col,
                format!(
                    "Expression at {}, {} should be on its own line.",
                    close_line,
                    close_col + 1
                ),
            ));
            return;
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(BlockEndNewline, "cops/layout/block_end_newline");
}
