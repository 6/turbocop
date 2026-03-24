use crate::cop::node_type::ARRAY_NODE;
use crate::cop::util::has_trailing_comma;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/TrailingCommaInArrayLiteral — checks trailing commas in array literals.
///
/// ## Heredoc handling
/// When arrays contain heredoc elements, Prism's `location().end_offset()` for
/// the last element points to the end of the heredoc opening tag (e.g., after
/// `<<~STR.chomp`), NOT the closing terminator. The heredoc body and terminator
/// sit between `last_end` and `closing_start` (`]`).
///
/// **Root cause of FPs:** Previous approach scanned from start of `]`'s line,
/// which could pick up commas inside heredoc content or terminators.
///
/// **Root cause of FNs:** Previous approach for multiline heredoc arrays scanned
/// from start of `]`'s line, missing the trailing comma on the heredoc opening
/// line (e.g., `<<~STR.chomp,`).
///
/// **Fix:** When heredocs are present, scan from `last_end` but stop at the
/// first newline (matching RuboCop's `/\A[^\S\n]*,/` regex). This finds commas
/// on the heredoc opening line without entering heredoc content.
///
/// ## Nested heredoc FPs (2026-03)
/// When the last element of an outer array is a sub-array containing a heredoc
/// (e.g., `["foo.rb", <<-EOS]`), the heredoc body sits between the sub-array's
/// `end_offset()` and the outer `]`. The `any_heredoc` check must recurse into
/// sub-arrays to detect these nested heredocs, otherwise heredoc content gets
/// scanned for commas producing false positives. Seen in zeitwerk, rufo, thredded.
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
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

        // For heredoc elements, Prism's location.end_offset() is at the
        // heredoc opening tag (e.g., `<<-RB`), not the closing tag. The
        // heredoc body and terminator sit between last_end and closing_start.
        //
        // RuboCop handles this by using a regex that disallows newlines when
        // heredocs are present: /\A[^\S\n]*,/ — only whitespace (no newlines)
        // before a comma. This ensures we find trailing commas on the heredoc
        // opening line (e.g., `<<~STR.chomp,`) but never match commas inside
        // heredoc content on subsequent lines.
        let has_heredoc = any_heredoc(&elements);
        let has_comma = if has_heredoc {
            has_trailing_comma_no_newline(bytes, last_end, closing_start)
        } else {
            has_trailing_comma(bytes, last_end, closing_start)
        };
        // For finding the comma offset in diagnostics, use the same logic
        let effective_last_end = last_end;

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

        // Helper: find the absolute offset of the trailing comma for diagnostics.
        // When heredocs are present, only search on the same line (no newline crossing).
        let find_comma_offset = || -> Option<usize> {
            let search_range = &bytes[effective_last_end..closing_start];
            for (i, &b) in search_range.iter().enumerate() {
                if has_heredoc && b == b'\n' {
                    return None; // Stop at newline for heredoc elements
                }
                if b == b',' {
                    return Some(effective_last_end + i);
                }
            }
            None
        };

        match style {
            "comma" => {
                let each_on_own_line =
                    no_elements_on_same_line(source, &elements[..], closing_start);
                let should_have = is_multiline && each_on_own_line;
                if has_comma && !should_have {
                    if let Some(abs_offset) = find_comma_offset() {
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
                    if let Some(abs_offset) = find_comma_offset() {
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
            "diff_comma" => {
                let last_precedes_newline =
                    is_multiline && last_item_precedes_newline(bytes, last_end, closing_start);
                if has_comma && !last_precedes_newline {
                    if let Some(abs_offset) = find_comma_offset() {
                        let (line, column) = source.offset_to_line_col(abs_offset);
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Avoid comma after the last item of an array, unless that item immediately precedes a newline.".to_string(),
                        ));
                    }
                } else if !has_comma && last_precedes_newline {
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
                    if let Some(abs_offset) = find_comma_offset() {
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

/// Like `has_trailing_comma` but stops at the first newline. Used when
/// heredocs are present: only match commas on the same line as the last
/// element's end offset (the heredoc opening tag line), never inside
/// heredoc content on subsequent lines. Matches RuboCop's `/\A[^\S\n]*,/`
/// regex used in `comma_offset` when `any_heredoc?` is true.
fn has_trailing_comma_no_newline(
    source_bytes: &[u8],
    last_element_end: usize,
    closing_start: usize,
) -> bool {
    if last_element_end >= closing_start || closing_start > source_bytes.len() {
        return false;
    }
    let region = &source_bytes[last_element_end..closing_start];
    for &b in region {
        if b == b'\n' {
            return false; // Stop at newline — don't enter heredoc content
        }
        if b == b',' {
            return true;
        }
        // Skip horizontal whitespace only
        if b != b' ' && b != b'\t' {
            return false; // Non-whitespace, non-comma, non-newline — no trailing comma
        }
    }
    false
}

/// Returns true if any element in the list is or contains a heredoc.
fn any_heredoc(elements: &[ruby_prism::Node<'_>]) -> bool {
    elements.iter().any(|e| is_heredoc_element(e))
}

/// Returns true if a node is a heredoc or wraps one (e.g., method call on heredoc).
fn is_heredoc_element(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(s) = node.as_interpolated_string_node() {
        if let Some(open) = s.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                return true;
            }
        }
    }
    if let Some(s) = node.as_string_node() {
        if let Some(open) = s.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                return true;
            }
        }
    }
    // Check method calls on heredocs (e.g., <<~SQL.strip.chomp)
    if let Some(call) = node.as_call_node() {
        if let Some(recv) = call.receiver() {
            return is_heredoc_element(&recv);
        }
    }
    // Check sub-arrays that may contain heredoc elements (e.g., ["foo.rb", <<-EOS])
    if let Some(arr) = node.as_array_node() {
        for child in arr.elements().iter() {
            if is_heredoc_element(&child) {
                return true;
            }
        }
    }
    false
}

/// Returns true if the last item immediately precedes a newline (possibly with
/// an optional comma and inline comment in between). Matches RuboCop's
/// `last_item_precedes_newline?` for the `diff_comma` style.
fn last_item_precedes_newline(bytes: &[u8], last_end: usize, closing_start: usize) -> bool {
    // Check the text after the last element: ,?\s*(#.*)?\n
    let region = &bytes[last_end..closing_start];
    let mut i = 0;
    // Skip optional comma
    if i < region.len() && region[i] == b',' {
        i += 1;
    }
    // Skip spaces/tabs (but not newlines)
    while i < region.len() && (region[i] == b' ' || region[i] == b'\t') {
        i += 1;
    }
    // Skip optional comment
    if i < region.len() && region[i] == b'#' {
        while i < region.len() && region[i] != b'\n' {
            i += 1;
        }
    }
    // Must end with a newline
    i < region.len() && region[i] == b'\n'
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
        let start_line = source
            .offset_to_line_col(pair[1].location().start_offset())
            .0;
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
        assert_cop_no_offenses_full_with_config(
            &TrailingCommaInArrayLiteral,
            fixture,
            comma_config(),
        );
    }

    #[test]
    fn comma_style_multiline_each_on_own_line_missing_comma_offense() {
        // Multiline array with each element on its own line, missing trailing comma
        let fixture = b"# nitrocop-expect: 4:3 Style/TrailingCommaInArrayLiteral: Put a comma after the last item of a multiline array.\nx = [\n  1,\n  2,\n  3\n]\n";
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
        assert_cop_no_offenses_full_with_config(
            &TrailingCommaInArrayLiteral,
            fixture,
            comma_config(),
        );
    }

    fn diff_comma_config() -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "EnforcedStyleForMultiline".to_string(),
            serde_yml::Value::String("diff_comma".to_string()),
        );
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn diff_comma_style_single_line_trailing_comma_offense() {
        let fixture = b"[1, 2, 3,]\n        ^ Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array, unless that item immediately precedes a newline.\n";
        assert_cop_offenses_full_with_config(
            &TrailingCommaInArrayLiteral,
            fixture,
            diff_comma_config(),
        );
    }

    #[test]
    fn diff_comma_style_multiline_last_on_own_line_missing_comma_offense() {
        // Last element is followed by newline — should require comma
        let fixture = b"# nitrocop-expect: 3:3 Style/TrailingCommaInArrayLiteral: Put a comma after the last item of a multiline array.\nx = [\n  1,\n  2\n]\n";
        assert_cop_offenses_full_with_config(
            &TrailingCommaInArrayLiteral,
            fixture,
            diff_comma_config(),
        );
    }

    #[test]
    fn diff_comma_style_multiline_with_comma_no_offense() {
        // Last element has trailing comma and precedes newline — fine
        let fixture = b"x = [\n  1,\n  2,\n]\n";
        assert_cop_no_offenses_full_with_config(
            &TrailingCommaInArrayLiteral,
            fixture,
            diff_comma_config(),
        );
    }

    #[test]
    fn diff_comma_style_multiline_elements_sharing_lines_with_comma_no_offense() {
        // Multiple elements per line, last element precedes newline, has comma
        let fixture = b"x = [\n  1, 2,\n  3,\n]\n";
        assert_cop_no_offenses_full_with_config(
            &TrailingCommaInArrayLiteral,
            fixture,
            diff_comma_config(),
        );
    }

    #[test]
    fn diff_comma_style_closing_on_same_line_trailing_comma_offense() {
        // Closing bracket on same line as last element — comma is unwanted
        let fixture = b"[1, 2,\n     3,]\n      ^ Style/TrailingCommaInArrayLiteral: Avoid comma after the last item of an array, unless that item immediately precedes a newline.\n";
        assert_cop_offenses_full_with_config(
            &TrailingCommaInArrayLiteral,
            fixture,
            diff_comma_config(),
        );
    }

    #[test]
    fn diff_comma_style_single_line_no_comma_no_offense() {
        let fixture = b"[1, 2, 3]\n";
        assert_cop_no_offenses_full_with_config(
            &TrailingCommaInArrayLiteral,
            fixture,
            diff_comma_config(),
        );
    }
}
