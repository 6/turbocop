use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct SpaceInsideReferenceBrackets;

impl Cop for SpaceInsideReferenceBrackets {
    fn name(&self) -> &'static str {
        "Layout/SpaceInsideReferenceBrackets"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // This cop checks [] and []= method calls (reference brackets)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        if method_name != b"[]" && method_name != b"[]=" {
            return;
        }

        // Must have a receiver (e.g., hash[:key], not standalone [])
        if call.receiver().is_none() {
            return;
        }

        let enforced_style = config.get_str("EnforcedStyle", "no_space");
        let empty_style = config.get_str("EnforcedStyleForEmptyBrackets", "no_space");

        let bytes = source.as_bytes();

        // Find the opening [ bracket position
        // For call nodes like hash[:key], the opening_loc gives us the (
        // But for [] calls, we need to find the [ in the source
        let opening_loc = match call.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        let closing_loc = match call.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        // Verify these are brackets
        if opening_loc.as_slice() != b"[" || closing_loc.as_slice() != b"]" {
            return;
        }

        let open_end = opening_loc.end_offset();
        let close_start = closing_loc.start_offset();

        // Skip multiline
        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing_loc.start_offset());
        if open_line != close_line {
            return;
        }


        // Check for empty brackets
        let is_empty = close_start == open_end
            || (close_start > open_end
                && bytes[open_end..close_start]
                    .iter()
                    .all(|&b| b == b' ' || b == b'\t'));

        if is_empty {
            match empty_style {
                "no_space" => {
                    if close_start > open_end {
                        let (line, col) = source.offset_to_line_col(open_end);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            col,
                            "Do not use space inside empty reference brackets.".to_string(),
                        ));
                    }
                }
                "space" => {
                    if close_start == open_end || (close_start - open_end) != 1 {
                        let (line, col) = source.offset_to_line_col(opening_loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            col,
                            "Use one space inside empty reference brackets.".to_string(),
                        ));
                    }
                }
                _ => {}
            }
            return;
        }

        let space_after_open = bytes.get(open_end) == Some(&b' ');
        let space_before_close = close_start > 0 && bytes.get(close_start - 1) == Some(&b' ');

        match enforced_style {
            "no_space" => {
                if space_after_open {
                    let (line, col) = source.offset_to_line_col(open_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Do not use space inside reference brackets.".to_string(),
                    ));
                }
                if space_before_close {
                    let (line, col) = source.offset_to_line_col(close_start - 1);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Do not use space inside reference brackets.".to_string(),
                    ));
                }
            }
            "space" => {
                if !space_after_open {
                    let (line, col) = source.offset_to_line_col(open_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Use space inside reference brackets.".to_string(),
                    ));
                }
                if !space_before_close {
                    let (line, col) = source.offset_to_line_col(close_start);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Use space inside reference brackets.".to_string(),
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
        SpaceInsideReferenceBrackets,
        "cops/layout/space_inside_reference_brackets"
    );
}
