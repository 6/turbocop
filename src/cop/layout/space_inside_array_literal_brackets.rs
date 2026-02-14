use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SpaceInsideArrayLiteralBrackets;

impl Cop for SpaceInsideArrayLiteralBrackets {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideArrayLiteralBrackets"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let opening = match array.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(), // Implicit array (no brackets)
        };
        let closing = match array.closing_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        // Only check [ ] arrays
        if opening.as_slice() != b"[" || closing.as_slice() != b"]" {
            return Vec::new();
        }

        let bytes = source.as_bytes();
        let open_end = opening.end_offset();
        let close_start = closing.start_offset();

        // Skip empty arrays []
        if close_start == open_end {
            return Vec::new();
        }

        // Skip multiline arrays
        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing.start_offset());
        if open_line != close_line {
            return Vec::new();
        }

        let enforced = config.get_str("EnforcedStyle", "no_space");

        let mut diagnostics = Vec::new();

        let space_after_open = bytes.get(open_end) == Some(&b' ');
        let space_before_close = close_start > 0 && bytes.get(close_start - 1) == Some(&b' ');

        match enforced {
            "no_space" => {
                if space_after_open {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside array literal brackets detected.".to_string(),
                    ));
                }
                if space_before_close {
                    let (line, column) = source.offset_to_line_col(closing.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside array literal brackets detected.".to_string(),
                    ));
                }
            }
            "space" => {
                if !space_after_open {
                    let (line, column) = source.offset_to_line_col(opening.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside array literal brackets missing.".to_string(),
                    ));
                }
                if !space_before_close {
                    let (line, column) = source.offset_to_line_col(closing.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Space inside array literal brackets missing.".to_string(),
                    ));
                }
            }
            _ => {}
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SpaceInsideArrayLiteralBrackets,
        "cops/layout/space_inside_array_literal_brackets"
    );
}
