use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::ARRAY_NODE;

pub struct MultilineArrayBraceLayout;

impl Cop for MultilineArrayBraceLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineArrayBraceLayout"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "symmetrical");

        let array = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let opening = match array.opening_loc() {
            Some(loc) => loc,
            None => return, // Implicit array
        };
        let closing = match array.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Only check bracket arrays
        if opening.as_slice() != b"[" || closing.as_slice() != b"]" {
            return;
        }

        let elements = array.elements();
        if elements.is_empty() {
            return;
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing.start_offset());

        // Get first and last element lines
        let first_elem = elements.iter().next().unwrap();
        let last_elem = elements.iter().last().unwrap();
        let (first_elem_line, _) = source.offset_to_line_col(first_elem.location().start_offset());
        let (last_elem_line, _) = source.offset_to_line_col(
            last_elem.location().end_offset().saturating_sub(1),
        );

        // Only check multiline arrays
        if open_line == close_line {
            return;
        }

        let open_same_as_first = open_line == first_elem_line;
        let close_same_as_last = close_line == last_elem_line;

        match enforced_style {
            "symmetrical" => {
                // Opening and closing should be symmetric
                if open_same_as_first && !close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "The closing array brace must be on the same line as the last array element when the opening brace is on the same line as the first array element.".to_string(),
                    ));
                }
                if !open_same_as_first && close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "The closing array brace must be on the line after the last array element when the opening brace is on a separate line from the first array element.".to_string(),
                    ));
                }
            }
            "new_line" => {
                if close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "The closing array brace must be on the line after the last array element."
                            .to_string(),
                    ));
                }
            }
            "same_line" => {
                if !close_same_as_last {
                    diagnostics.push(self.diagnostic(
                        source,
                        close_line,
                        close_col,
                        "The closing array brace must be on the same line as the last array element."
                            .to_string(),
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
        MultilineArrayBraceLayout,
        "cops/layout/multiline_array_brace_layout"
    );
}
