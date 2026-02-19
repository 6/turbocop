use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::BLOCK_NODE;

pub struct SingleLineDoEndBlock;

impl Cop for SingleLineDoEndBlock {
    fn name(&self) -> &'static str {
        "Style/SingleLineDoEndBlock"
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
    ) -> Vec<Diagnostic> {
        let block = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Check if it uses do...end
        let open_loc = block.opening_loc();
        if open_loc.as_slice() != b"do" {
            return Vec::new();
        }

        // Check if block is on single line
        let (start_line, _) = source.offset_to_line_col(block.location().start_offset());
        let (end_line, _) = source.offset_to_line_col(block.location().end_offset());
        if start_line != end_line {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(open_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer braces `{...}` over `do...end` for single-line blocks.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SingleLineDoEndBlock, "cops/style/single_line_do_end_block");
}
