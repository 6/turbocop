use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsideStringInterpolation;

impl Cop for SpaceInsideStringInterpolation {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideStringInterpolation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "no_space");

        // EmbeddedStatementsNode represents `#{ ... }` inside strings
        let embedded = match node.as_embedded_statements_node() {
            Some(e) => e,
            None => return Vec::new(),
        };

        let open_loc = embedded.opening_loc();
        let close_loc = embedded.closing_loc();

        let (open_line, _) = source.offset_to_line_col(open_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(close_loc.start_offset());

        // Skip multiline interpolations
        if open_line != close_line {
            return Vec::new();
        }

        let bytes = source.as_bytes();
        let open_end = open_loc.end_offset(); // position after `#{`
        let close_start = close_loc.start_offset(); // position of `}`

        // Skip empty interpolation
        if close_start <= open_end {
            return Vec::new();
        }

        let space_after_open = bytes.get(open_end) == Some(&b' ');
        let space_before_close = close_start > 0 && bytes.get(close_start - 1) == Some(&b' ');

        let mut diagnostics = Vec::new();

        match style {
            "space" => {
                // Require spaces
                if !space_after_open {
                    let (line, col) = source.offset_to_line_col(open_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Missing space inside string interpolation.".to_string(),
                    ));
                }
                if !space_before_close {
                    let (line, col) = source.offset_to_line_col(close_start);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Missing space inside string interpolation.".to_string(),
                    ));
                }
            }
            _ => {
                // "no_space" (default) — flag spaces
                if space_after_open {
                    let (line, col) = source.offset_to_line_col(open_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Space inside string interpolation detected.".to_string(),
                    ));
                }
                if space_before_close {
                    let (line, col) = source.offset_to_line_col(close_start - 1);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Space inside string interpolation detected.".to_string(),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceInsideStringInterpolation,
        "cops/layout/space_inside_string_interpolation"
    );
}
