use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE};

pub struct TrailingCommaInBlockArgs;

impl Cop for TrailingCommaInBlockArgs {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInBlockArgs"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE]
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
        let block = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let params = match block.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };

        // Check the source for a trailing comma before |
        let close_loc = match block_params.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Look at bytes before the closing |
        let bytes = source.as_bytes();
        let close_offset = close_loc.start_offset();
        if close_offset == 0 {
            return;
        }

        // Scan backwards for trailing comma (skip whitespace)
        let mut pos = close_offset - 1;
        while pos > 0 && (bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n') {
            pos -= 1;
        }

        if bytes[pos] == b',' {
            let (line, column) = source.offset_to_line_col(pos);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Useless trailing comma present in block arguments.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TrailingCommaInBlockArgs, "cops/style/trailing_comma_in_block_args");
}
