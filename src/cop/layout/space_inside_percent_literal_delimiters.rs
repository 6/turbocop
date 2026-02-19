use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::ARRAY_NODE;

pub struct SpaceInsidePercentLiteralDelimiters;

impl Cop for SpaceInsidePercentLiteralDelimiters {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsidePercentLiteralDelimiters"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check array nodes that are %w or %i style
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
        // Check if this is a percent literal (%w, %W, %i, %I)
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

        // Check for leading spaces
        if !content.is_empty() && content[0] == b' ' {
            let (line, col) = source.offset_to_line_col(open_end);
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Do not use spaces inside percent literal delimiters.".to_string(),
            ));
        }

        // Check for trailing spaces
        if content.len() > 1 && content[content.len() - 1] == b' ' {
            let trailing_start = close_start - 1;
            let (line, col) = source.offset_to_line_col(trailing_start);
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                "Do not use spaces inside percent literal delimiters.".to_string(),
            ));
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceInsidePercentLiteralDelimiters,
        "cops/layout/space_inside_percent_literal_delimiters"
    );
}
