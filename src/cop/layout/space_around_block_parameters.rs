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

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<crate::correction::Correction>>,
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
                    let mut diag = self.diagnostic(
                        source, line, col,
                        "Space before first block parameter detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: start + 1, end: start + 2, replacement: String::new(),
                            cop_name: self.name(), cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                // Check for space before closing pipe
                if close_pipe > start + 1 && bytes[close_pipe - 1] == b' ' {
                    let (line, col) = source.offset_to_line_col(close_pipe - 1);
                    let mut diag = self.diagnostic(
                        source, line, col,
                        "Space after last block parameter detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: close_pipe - 1, end: close_pipe, replacement: String::new(),
                            cop_name: self.name(), cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            }
            "space" => {
                // Check for missing space after opening pipe
                if start + 1 < close_pipe && bytes[start + 1] != b' ' {
                    let (line, col) = source.offset_to_line_col(start + 1);
                    let mut diag = self.diagnostic(
                        source, line, col,
                        "No space before first block parameter detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: start + 1, end: start + 1, replacement: " ".to_string(),
                            cop_name: self.name(), cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
                // Check for missing space before closing pipe
                if close_pipe > start + 1 && bytes[close_pipe - 1] != b' ' {
                    let (line, col) = source.offset_to_line_col(close_pipe);
                    let mut diag = self.diagnostic(
                        source, line, col,
                        "No space after last block parameter detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: close_pipe, end: close_pipe, replacement: " ".to_string(),
                            cop_name: self.name(), cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
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
    crate::cop_autocorrect_fixture_tests!(
        SpaceAroundBlockParameters,
        "cops/layout/space_around_block_parameters"
    );
}
