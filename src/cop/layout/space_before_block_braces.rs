use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceBeforeBlockBraces;

impl Cop for SpaceBeforeBlockBraces {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeBlockBraces"
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

        let opening = block.opening_loc();

        // Only check { blocks, not do...end
        if opening.as_slice() != b"{" {
            return Vec::new();
        }

        let bytes = source.as_bytes();
        let before = opening.start_offset();
        if before > 0 && bytes[before - 1] != b' ' {
            let (line, column) = source.offset_to_line_col(before);
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Space missing to the left of {.".to_string(),
            )];
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeBlockBraces, "cops/layout/space_before_block_braces");
}
