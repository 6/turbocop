use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::BLOCK_NODE;

pub struct SpaceAroundBlockParameters;

impl Cop for SpaceAroundBlockParameters {
    fn name(&self) -> &'static str {
        "Layout/SpaceAroundBlockParameters"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let style = config.get_str("EnforcedStyleInsidePipes", "no_space");

        let block = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let params = match block.parameters() {
            Some(p) => p,
            None => return,
        };

        let params_loc = params.location();
        let bytes = source.as_bytes();
        let start = params_loc.start_offset();
        let end = params_loc.end_offset();

        // Params location should be |...|
        if start >= end || start >= bytes.len() || end > bytes.len() {
            return;
        }
        if bytes[start] != b'|' {
            return;
        }

        // Find the closing pipe
        let close_pipe = end.saturating_sub(1);
        if close_pipe <= start || bytes[close_pipe] != b'|' {
            return;
        }


        match style {
            "no_space" => {
                // Check for space after opening pipe
                if start + 1 < close_pipe && bytes[start + 1] == b' ' {
                    let (line, col) = source.offset_to_line_col(start + 1);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Space before first block parameter detected.".to_string(),
                    ));
                }
                // Check for space before closing pipe
                if close_pipe > start + 1 && bytes[close_pipe - 1] == b' ' {
                    let (line, col) = source.offset_to_line_col(close_pipe - 1);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Space after last block parameter detected.".to_string(),
                    ));
                }
            }
            "space" => {
                // Check for missing space after opening pipe
                if start + 1 < close_pipe && bytes[start + 1] != b' ' {
                    let (line, col) = source.offset_to_line_col(start + 1);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "No space before first block parameter detected.".to_string(),
                    ));
                }
                // Check for missing space before closing pipe
                if close_pipe > start + 1 && bytes[close_pipe - 1] != b' ' {
                    let (line, col) = source.offset_to_line_col(close_pipe);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "No space after last block parameter detected.".to_string(),
                    ));
                }
            }
            _ => {}
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceAroundBlockParameters,
        "cops/layout/space_around_block_parameters"
    );
}
