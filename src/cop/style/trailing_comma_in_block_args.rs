use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingCommaInBlockArgs;

impl Cop for TrailingCommaInBlockArgs {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInBlockArgs"
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

        let params = match block.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return Vec::new(),
        };

        // Check the source for a trailing comma before |
        let close_loc = match block_params.closing_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        // Look at bytes before the closing |
        let bytes = source.as_bytes();
        let close_offset = close_loc.start_offset();
        if close_offset == 0 {
            return Vec::new();
        }

        // Scan backwards for trailing comma (skip whitespace)
        let mut pos = close_offset - 1;
        while pos > 0 && (bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n') {
            pos -= 1;
        }

        if bytes[pos] == b',' {
            let (line, column) = source.offset_to_line_col(pos);
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Useless trailing comma present in block arguments.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrailingCommaInBlockArgs, "cops/style/trailing_comma_in_block_args");
}
