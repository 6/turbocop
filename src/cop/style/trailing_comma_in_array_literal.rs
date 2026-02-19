use crate::cop::util::has_trailing_comma;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::ARRAY_NODE;

pub struct TrailingCommaInArrayLiteral;

impl Cop for TrailingCommaInArrayLiteral {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInArrayLiteral"
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
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return,
        };

        // Skip %w(), %W(), %i(), %I() word/symbol arrays — they don't use commas
        if let Some(opening) = array_node.opening_loc() {
            if source.as_bytes().get(opening.start_offset()) == Some(&b'%') {
                return;
            }
        }

        let closing_loc = match array_node.closing_loc() {
            Some(loc) => loc,
            None => return,
        };

        let elements: Vec<ruby_prism::Node<'_>> = array_node.elements().iter().collect();
        let last_elem = match elements.last() {
            Some(e) => e,
            None => return,
        };

        let last_end = last_elem.location().end_offset();
        let closing_start = closing_loc.start_offset();
        let bytes = source.as_bytes();

        // For heredoc elements (or method calls on heredocs), the node's
        // location.end_offset() might be on the opening line (before the
        // heredoc body), while the closing `]` is after the heredoc body.
        // In this case, scanning from last_end to closing_start would scan
        // through heredoc content and find false commas. Instead, scan only
        // the line just before the closing bracket for a trailing comma.
        let effective_last_end = {
            let closing_line = source.offset_to_line_col(closing_start).0;
            let last_end_line = source.offset_to_line_col(last_end).0;
            if closing_line > last_end_line + 1 {
                // Gap between last element end and closing bracket -- likely heredoc content.
                // Scan from the start of the closing bracket's line instead.
                let mut pos = closing_start;
                while pos > 0 && bytes[pos - 1] != b'\n' {
                    pos -= 1;
                }
                pos
            } else {
                last_end
            }
        };
        let has_comma = has_trailing_comma(bytes, effective_last_end, closing_start);

        let style = config.get_str("EnforcedStyleForMultiline", "no_comma");

        // Check if array is multiline: the opening `[` and closing `]` are on different lines.
        // Also handle allowed_multiline_argument: single element with `]` on same line as element end.
        let open_line = if let Some(opening) = array_node.opening_loc() {
            source.offset_to_line_col(opening.start_offset()).0
        } else {
            return;
        };
        let close_line = source.offset_to_line_col(closing_start).0;
        let is_multiline = if elements.len() == 1 {
            // Single element: only consider multiline if closing bracket is on a different line
            // than the end of the element (allowed_multiline_argument check)
            let last_line = source.offset_to_line_col(last_end).0;
            close_line > last_line
        } else {
            close_line > open_line
        };

        match style {
            "comma" => {
                let each_on_own_line = no_elements_on_same_line(source, &elements[..], closing_start);
                let should_have = is_multiline && each_on_own_line;
                if has_comma && !should_have {
                    // Trailing comma present but not wanted (single-line or elements share lines)
                    let search_range = &bytes[last_end..closing_start];
                    if let Some(comma_offset) =
                        search_range.iter().position(|&b| b == b',')
                    {
                        let abs_offset = last_end + comma_offset;
                        let (line, column) = source.offset_to_line_col(abs_offset);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid comma after the last item of an array, unless each item is on its own line.".to_string(),
                        ));
                    }
                } else if !has_comma && should_have {
                    let (line, column) = source.offset_to_line_col(last_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Put a comma after the last item of a multiline array.".to_string(),
                    ));
                }
            }
            "consistent_comma" => {
                if has_comma && !is_multiline {
                    // Trailing comma on single-line array
                    let search_range = &bytes[last_end..closing_start];
                    if let Some(comma_offset) =
                        search_range.iter().position(|&b| b == b',')
                    {
                        let abs_offset = last_end + comma_offset;
                        let (line, column) = source.offset_to_line_col(abs_offset);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid comma after the last item of an array, unless items are split onto multiple lines.".to_string(),
                        ));
                    }
                } else if !has_comma && is_multiline {
                    let (line, column) = source.offset_to_line_col(last_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Put a comma after the last item of a multiline array.".to_string(),
                    ));
                }
            }
            _ => {
                if has_comma {
                    let search_range = &bytes[last_end..closing_start];
                    if let Some(comma_offset) =
                        search_range.iter().position(|&b| b == b',')
                    {
                        let abs_offset = last_end + comma_offset;
                        let (line, column) = source.offset_to_line_col(abs_offset);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid comma after the last item of an array.".to_string(),
                        ));
                    }
                }
            }
        }

    }
}

/// Returns true if no two consecutive items (including the closing bracket)
/// are on the same line. This matches RuboCop's `no_elements_on_same_line?`.
fn no_elements_on_same_line(
    source: &SourceFile,
    elements: &[ruby_prism::Node<'_>],
    closing_start: usize,
) -> bool {
    // Check each consecutive pair of elements
    for pair in elements.windows(2) {
        let end_line = source.offset_to_line_col(pair[0].location().end_offset()).0;
        let start_line = source.offset_to_line_col(pair[1].location().start_offset()).0;
        if end_line == start_line {
            return false;
        }
    }
    // Check last element end vs closing bracket start
    if let Some(last) = elements.last() {
        let last_end_line = source.offset_to_line_col(last.location().end_offset()).0;
        let close_line = source.offset_to_line_col(closing_start).0;
        if last_end_line == close_line {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use crate::testutil::{
        assert_cop_no_offenses_full_with_config, assert_cop_offenses_full_with_config,
    };
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        TrailingCommaInArrayLiteral,
        "cops/style/trailing_comma_in_array_literal"
    );

    fn comma_config() -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "EnforcedStyleForMultiline".to_string(),
            serde_yml::Value::String("comma".to_string()),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn comma_style_multiline_elements_on_same_line_no_offense() {
        // Multiline array with elements sharing lines should NOT be flagged
        let fixture = b"x = [\n  Foo, Bar, Baz,\n  Qux\n]\n";
        assert_cop_no_offenses_full_with_config(&TrailingCommaInArrayLiteral, fixture, comma_config());
    }

    #[test]
    fn comma_style_multiline_each_on_own_line_missing_comma_offense() {
        // Multiline array with each element on its own line, missing trailing comma
        let fixture = b"# rblint-expect: 4:3 Style/TrailingCommaInArrayLiteral: Put a comma after the last item of a multiline array.\nx = [\n  1,\n  2,\n  3\n]\n";
        assert_cop_offenses_full_with_config(&TrailingCommaInArrayLiteral, fixture, comma_config());
    }

    #[test]
    fn comma_style_single_line_trailing_comma_offense() {
        // Single-line array with trailing comma should be flagged even in comma style
        let fixture = b"[1, 2, 3,]\n        ^ Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array, unless each item is on its own line.\n";
        assert_cop_offenses_full_with_config(&TrailingCommaInArrayLiteral, fixture, comma_config());
    }

    #[test]
    fn comma_style_multiline_each_on_own_line_with_comma_no_offense() {
        // Multiline array with each element on its own line AND trailing comma is fine
        let fixture = b"x = [\n  1,\n  2,\n  3,\n]\n";
        assert_cop_no_offenses_full_with_config(&TrailingCommaInArrayLiteral, fixture, comma_config());
    }
}
