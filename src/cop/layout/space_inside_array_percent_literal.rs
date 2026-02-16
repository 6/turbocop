use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsideArrayPercentLiteral;

impl Cop for SpaceInsideArrayPercentLiteral {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideArrayPercentLiteral"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let open_loc = match array.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let close_loc = match array.closing_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let open_slice = open_loc.as_slice();
        // Only percent array literals
        if !open_slice.starts_with(b"%w") && !open_slice.starts_with(b"%W")
            && !open_slice.starts_with(b"%i") && !open_slice.starts_with(b"%I")
        {
            return Vec::new();
        }

        let bytes = source.as_bytes();
        let open_end = open_loc.end_offset();
        let close_start = close_loc.start_offset();

        // Skip multiline
        let (open_line, _) = source.offset_to_line_col(open_end.saturating_sub(1));
        let (close_line, _) = source.offset_to_line_col(close_start);
        if open_line != close_line {
            return Vec::new();
        }

        if close_start <= open_end {
            return Vec::new();
        }

        let content = &bytes[open_end..close_start];
        let mut diagnostics = Vec::new();

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
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            col,
                            "Use only a single space inside array percent literal.".to_string(),
                        ));
                    }
                }
            } else {
                i += 1;
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceInsideArrayPercentLiteral,
        "cops/layout/space_inside_array_percent_literal"
    );
}
