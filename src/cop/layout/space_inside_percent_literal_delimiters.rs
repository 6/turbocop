use crate::cop::node_type::ARRAY_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsidePercentLiteralDelimiters;

impl Cop for SpaceInsidePercentLiteralDelimiters {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsidePercentLiteralDelimiters"
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
        // Check array nodes that are %w or %i style
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
        // Check if this is a percent literal (%w, %W, %i, %I)
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

        // Check for leading spaces
        if !content.is_empty() && content[0] == b' ' {
            let (line, col) = source.offset_to_line_col(open_end);
            let mut diag = self.diagnostic(
                source,
                line,
                col,
                "Do not use spaces inside percent literal delimiters.".to_string(),
            );
            if let Some(ref mut corr) = corrections {
                // Count leading spaces
                let leading_count = content.iter().take_while(|&&b| b == b' ').count();
                corr.push(crate::correction::Correction {
                    start: open_end,
                    end: open_end + leading_count,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }

        // Check for trailing spaces
        if content.len() > 1 && content[content.len() - 1] == b' ' {
            let trailing_count = content.iter().rev().take_while(|&&b| b == b' ').count();
            let trailing_start = close_start - trailing_count;
            let (line, col) = source.offset_to_line_col(close_start - 1);
            let mut diag = self.diagnostic(
                source,
                line,
                col,
                "Do not use spaces inside percent literal delimiters.".to_string(),
            );
            if let Some(ref mut corr) = corrections {
                corr.push(crate::correction::Correction {
                    start: trailing_start,
                    end: close_start,
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceInsidePercentLiteralDelimiters,
        "cops/layout/space_inside_percent_literal_delimiters"
    );
    crate::cop_autocorrect_fixture_tests!(
        SpaceInsidePercentLiteralDelimiters,
        "cops/layout/space_inside_percent_literal_delimiters"
    );
}
