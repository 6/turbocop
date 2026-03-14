use crate::cop::node_type::{ARRAY_NODE, INTERPOLATED_X_STRING_NODE, X_STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for unnecessary additional spaces inside the delimiters of
/// %i/%w/%x literals.
///
/// FN root cause (58 FNs): Only handled ArrayNode (%w/%W/%i/%I) but not
/// XStringNode / InterpolatedXStringNode (%x). RuboCop's `on_xstr` handler
/// processes %x literals separately. Added both x-string node types to
/// interested_node_types and extract open/close locations for all three
/// node families.
pub struct SpaceInsidePercentLiteralDelimiters;

/// Check for spaces inside percent literal delimiters given the opening
/// and closing byte offsets.
#[allow(clippy::too_many_arguments)]
fn check_percent_literal(
    cop: &SpaceInsidePercentLiteralDelimiters,
    source: &SourceFile,
    open_end: usize,
    close_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
) {
    let bytes = source.as_bytes();

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
        let mut diag = cop.diagnostic(
            source,
            line,
            col,
            "Do not use spaces inside percent literal delimiters.".to_string(),
        );
        if let Some(corr) = corrections.as_mut() {
            // Count leading spaces
            let leading_count = content.iter().take_while(|&&b| b == b' ').count();
            corr.push(crate::correction::Correction {
                start: open_end,
                end: open_end + leading_count,
                replacement: String::new(),
                cop_name: cop.name(),
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
        let mut diag = cop.diagnostic(
            source,
            line,
            col,
            "Do not use spaces inside percent literal delimiters.".to_string(),
        );
        if let Some(corr) = corrections.as_mut() {
            corr.push(crate::correction::Correction {
                start: trailing_start,
                end: close_start,
                replacement: String::new(),
                cop_name: cop.name(),
                cop_index: 0,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

impl Cop for SpaceInsidePercentLiteralDelimiters {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsidePercentLiteralDelimiters"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, X_STRING_NODE, INTERPOLATED_X_STRING_NODE]
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
        // Extract open/close locations depending on node type
        if let Some(array) = node.as_array_node() {
            // %w, %W, %i, %I array literals
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

            check_percent_literal(
                self,
                source,
                open_loc.end_offset(),
                close_loc.start_offset(),
                diagnostics,
                &mut corrections,
            );
        } else if let Some(xstr) = node.as_x_string_node() {
            // %x() command literals (no interpolation)
            let open_loc = xstr.opening_loc();
            let close_loc = xstr.closing_loc();

            let open_slice = open_loc.as_slice();
            // Only handle %x style, not backtick style
            if !open_slice.starts_with(b"%x") {
                return;
            }

            check_percent_literal(
                self,
                source,
                open_loc.end_offset(),
                close_loc.start_offset(),
                diagnostics,
                &mut corrections,
            );
        } else if let Some(ixstr) = node.as_interpolated_x_string_node() {
            // %x() command literals (with interpolation)
            let open_loc = ixstr.opening_loc();
            let close_loc = ixstr.closing_loc();

            let open_slice = open_loc.as_slice();
            // Only handle %x style, not backtick style
            if !open_slice.starts_with(b"%x") {
                return;
            }

            check_percent_literal(
                self,
                source,
                open_loc.end_offset(),
                close_loc.start_offset(),
                diagnostics,
                &mut corrections,
            );
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
