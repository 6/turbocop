use crate::cop::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsideArrayPercentLiteral;

impl Cop for SpaceInsideArrayPercentLiteral {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideArrayPercentLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let array = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let open_loc = match array.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        let close_loc = match array.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        let open_slice = open_loc.as_slice();
        // Only percent array literals
        if !open_slice.starts_with(b"%w")
            && !open_slice.starts_with(b"%W")
            && !open_slice.starts_with(b"%i")
            && !open_slice.starts_with(b"%I")
        {
            return;
        }

        let bytes = source.as_bytes();
        let open_end = open_loc.end_offset();
        let close_start = close_loc.start_offset();

        // Skip multiline
        let (open_line, _) = source.offset_to_line_col(open_end.saturating_sub(1));
        let (close_line, _) = source.offset_to_line_col(close_start);
        if open_line != close_line {
            return;
        }

        if close_start <= open_end {
            return;
        }

        let content = &bytes[open_end..close_start];

        // Find multiple consecutive spaces between non-space characters
        let mut i = 0;
        while i < content.len() {
            // Find a space
            if content[i] == b' ' {
                let space_start = i;
                while i < content.len() && content[i] == b' ' {
                    i += 1;
                }
                let space_len = i - space_start;
                // Multiple spaces between items (not leading/trailing)
                if space_len >= 2 && space_start > 0 && i < content.len() {
                    // Check that character before spaces is not escaped
                    let prev_char = content[space_start - 1];
                    if prev_char != b'\\' {
                        let offset = open_end + space_start;
                        let (line, col) = source.offset_to_line_col(offset);
                        let mut diag = self.diagnostic(
                            source,
                            line,
                            col,
                            "Use only a single space inside array percent literal.".to_string(),
                        );
                        if let Some(ref mut corr) = corrections {
                            corr.push(crate::correction::Correction {
                                start: offset,
                                end: offset + space_len,
                                replacement: " ".to_string(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                        diagnostics.push(diag);
                    }
                }
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceInsideArrayPercentLiteral,
        "cops/layout/space_inside_array_percent_literal"
    );
    crate::cop_autocorrect_fixture_tests!(
        SpaceInsideArrayPercentLiteral,
        "cops/layout/space_inside_array_percent_literal"
    );
}
