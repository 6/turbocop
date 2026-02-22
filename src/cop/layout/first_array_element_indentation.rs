use crate::cop::node_type::ARRAY_NODE;
use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FirstArrayElementIndentation;

/// Describes what the expected indentation is relative to.
#[derive(Clone, Copy)]
enum IndentBaseType {
    /// `align_brackets` style: relative to the opening bracket `[`
    LeftBracket,
    /// `special_inside_parentheses`: relative to the first position after `(`
    FirstColumnAfterLeftParenthesis,
    /// Default: relative to the start of the line where `[` appears
    StartOfLine,
}

/// Scan backwards from `bracket_col` on `line_bytes` to find an unmatched `(`
/// that directly contains this array (not separated by an unmatched `[` or `{`).
/// Returns `Some(column)` if found, `None` otherwise.
///
/// This tracks balanced parens, brackets, AND braces. If we encounter an
/// unmatched `[` or `{` before finding an unmatched `(`, the array is nested
/// inside another array or a hash literal, not a direct argument of the
/// method call, so we return `None`.
fn find_left_paren_on_line(line_bytes: &[u8], bracket_col: usize) -> Option<usize> {
    let end = bracket_col.min(line_bytes.len());
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    for i in (0..end).rev() {
        match line_bytes[i] {
            b')' => paren_depth += 1,
            b'(' => {
                if paren_depth == 0 {
                    // Found an unmatched `(`. Only return it if we haven't
                    // passed through an unmatched `[` or `{` (which would mean
                    // our array is nested inside another array or hash literal).
                    if bracket_depth == 0 && brace_depth == 0 {
                        return Some(i);
                    }
                    return None;
                }
                paren_depth -= 1;
            }
            b']' => bracket_depth += 1,
            b'[' => {
                if bracket_depth == 0 {
                    // Unmatched `[` -- our array is inside another array.
                    return None;
                }
                bracket_depth -= 1;
            }
            b'}' => brace_depth += 1,
            b'{' => {
                if brace_depth == 0 {
                    // Unmatched `{` -- our array is inside a hash literal or block.
                    return None;
                }
                brace_depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Check if the array is the value of a hash pair (e.g. `key: [...]`).
/// If so, return the column of the hash key. This implements RuboCop's
/// `hash_pair_where_value_beginning_with` / `:parent_hash_key` logic.
///
/// Scans backwards from the `[` position on the same line to find a hash
/// key-value separator (`: ` or ` => `).
fn find_hash_pair_key_col(line_bytes: &[u8], bracket_col: usize) -> Option<usize> {
    if bracket_col == 0 {
        return None;
    }
    // Scan backwards from the bracket, skip whitespace
    let mut i = bracket_col - 1;
    while i > 0 && (line_bytes[i] == b' ' || line_bytes[i] == b'\t') {
        i -= 1;
    }
    // Check for symbol-style key `: ` pattern — the byte at i should be `:`
    // But NOT `::` (constant path)
    if line_bytes[i] == b':' && (i == 0 || line_bytes[i - 1] != b':') {
        // Found `key: [` — scan backwards from `:` to find the key start
        if i == 0 {
            return None;
        }
        let mut key_end = i - 1;
        // Skip whitespace between key and `:` for `key :` style (rocket `=>` is separate)
        while key_end > 0 && (line_bytes[key_end] == b' ' || line_bytes[key_end] == b'\t') {
            key_end -= 1;
        }
        // Key identifier: word chars
        let mut key_start = key_end;
        while key_start > 0
            && (line_bytes[key_start - 1].is_ascii_alphanumeric()
                || line_bytes[key_start - 1] == b'_')
        {
            key_start -= 1;
        }
        if key_start <= key_end
            && (line_bytes[key_start].is_ascii_alphabetic() || line_bytes[key_start] == b'_')
        {
            return Some(key_start);
        }
    }
    // Check for hashrocket `=> [` pattern
    if i >= 1 && line_bytes[i] == b'>' && line_bytes[i - 1] == b'=' {
        // Found `=> ` — scan backwards from `=` to find the key start
        let mut j = i - 2;
        while j > 0 && (line_bytes[j] == b' ' || line_bytes[j] == b'\t') {
            j -= 1;
        }
        // Key could be a symbol `:foo`, string `"foo"`, or identifier
        // For simplicity, find the first non-whitespace start of the key
        let mut key_start = j;
        while key_start > 0
            && line_bytes[key_start - 1] != b' '
            && line_bytes[key_start - 1] != b'\t'
            && line_bytes[key_start - 1] != b'{'
            && line_bytes[key_start - 1] != b','
        {
            key_start -= 1;
        }
        return Some(key_start);
    }
    None
}

/// Check if the array has a right sibling hash pair on a subsequent line.
/// Looks at bytes after the array's closing `]` for a `,` followed by
/// a newline (indicating more hash pairs follow).
fn has_right_sibling_on_next_line(source_bytes: &[u8], closing_end_offset: usize) -> bool {
    let mut i = closing_end_offset;
    let len = source_bytes.len();
    // Skip whitespace after `]`
    while i < len && (source_bytes[i] == b' ' || source_bytes[i] == b'\t') {
        i += 1;
    }
    // Expect a comma followed eventually by a newline (right sibling exists)
    if i < len && source_bytes[i] == b',' {
        i += 1;
        while i < len && (source_bytes[i] == b' ' || source_bytes[i] == b'\t') {
            i += 1;
        }
        if i >= len || source_bytes[i] == b'\n' || source_bytes[i] == b'\r' {
            return true;
        }
    }
    false
}

/// Check if the array is used as a direct argument (not as a receiver of
/// a method chain or part of a binary expression). Checks the source bytes
/// immediately after the array's closing bracket `]`.
///
/// Returns `true` if the array is a standalone argument (next non-whitespace
/// after `]` is `)`, `,`, end of line, or nothing relevant).
/// Returns `false` if `]` is followed by `.`, `+`, `-`, `*`, etc. indicating
/// the array is part of a larger expression.
fn is_direct_argument(source_bytes: &[u8], closing_end_offset: usize) -> bool {
    let mut i = closing_end_offset;
    let len = source_bytes.len();
    // Skip whitespace (but not newlines)
    while i < len && (source_bytes[i] == b' ' || source_bytes[i] == b'\t') {
        i += 1;
    }
    if i >= len {
        return true;
    }
    match source_bytes[i] {
        // Array is followed by a method call or operator => part of expression
        b'.' | b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' => false,
        // Everything else (closing paren, comma, newline, etc.) => direct argument
        _ => true,
    }
}

impl Cop for FirstArrayElementIndentation {
    fn name(&self) -> &'static str {
        "Layout/FirstArrayElementIndentation"
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

        let opening_loc = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        let elements: Vec<_> = array_node.elements().iter().collect();
        if elements.is_empty() {
            return;
        }

        let first_element = &elements[0];

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let first_loc = first_element.location();
        let (elem_line, elem_col) = source.offset_to_line_col(first_loc.start_offset());

        // Skip if first element is on same line as opening bracket
        if elem_line == open_line {
            return;
        }

        let style = config.get_str("EnforcedStyle", "special_inside_parentheses");
        let width = config.get_usize("IndentationWidth", 2);

        // Get the indentation of the line where `[` appears
        let open_line_bytes = source.lines().nth(open_line - 1).unwrap_or(b"");
        let open_line_indent = indentation_of(open_line_bytes);
        let (_, open_col) = source.offset_to_line_col(opening_loc.start_offset());

        // Compute the indent base column (before adding width) and its type.
        // The first element should be at `indent_base + width`.
        // The closing bracket should be at `indent_base`.
        let (indent_base, base_type) = match style {
            "consistent" => (open_line_indent, IndentBaseType::StartOfLine),
            "align_brackets" => (open_col, IndentBaseType::LeftBracket),
            _ => {
                // "special_inside_parentheses" (default):
                let closing_end = array_node
                    .closing_loc()
                    .map(|loc| loc.end_offset())
                    .unwrap_or(0);

                if let Some(paren_col) = find_left_paren_on_line(open_line_bytes, open_col) {
                    // If the `[` is on the same line as a method call's `(`,
                    // and the array is a direct argument (not part of a chain
                    // like `[...].join()` or `[...] + other`), indent relative
                    // to the position after `(`.
                    if is_direct_argument(source.as_bytes(), closing_end) {
                        (
                            paren_col + 1,
                            IndentBaseType::FirstColumnAfterLeftParenthesis,
                        )
                    } else {
                        (open_line_indent, IndentBaseType::StartOfLine)
                    }
                } else {
                    (open_line_indent, IndentBaseType::StartOfLine)
                }
            }
        };

        let expected_elem = indent_base + width;

        if elem_col != expected_elem {
            let base_description = match base_type {
                IndentBaseType::LeftBracket => "the position of the opening bracket",
                IndentBaseType::FirstColumnAfterLeftParenthesis => {
                    "the first position after the preceding left parenthesis"
                }
                IndentBaseType::StartOfLine => {
                    "the start of the line where the left square bracket is"
                }
            };
            diagnostics.push(self.diagnostic(
                source,
                elem_line,
                elem_col,
                format!(
                    "Use {} spaces for indentation in an array, relative to {}.",
                    width, base_description
                ),
            ));
        }

        // Check closing bracket indentation
        if let Some(closing_loc) = array_node.closing_loc() {
            let (close_line, close_col) = source.offset_to_line_col(closing_loc.start_offset());

            // Only check if the closing bracket is on its own line
            // (no non-whitespace characters before it on that line)
            let close_line_bytes = source.lines().nth(close_line - 1).unwrap_or(b"");
            let only_whitespace_before = close_line_bytes[..close_col.min(close_line_bytes.len())]
                .iter()
                .all(|&b| b == b' ' || b == b'\t');

            if only_whitespace_before && close_col != indent_base {
                let msg = match base_type {
                    IndentBaseType::LeftBracket => {
                        "Indent the right bracket the same as the left bracket.".to_string()
                    }
                    IndentBaseType::FirstColumnAfterLeftParenthesis => {
                        "Indent the right bracket the same as the first position \
                         after the preceding left parenthesis."
                            .to_string()
                    }
                    IndentBaseType::StartOfLine => {
                        "Indent the right bracket the same as the start of the line \
                         where the left bracket is."
                            .to_string()
                    }
                };
                diagnostics.push(self.diagnostic(source, close_line, close_col, msg));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        FirstArrayElementIndentation,
        "cops/layout/first_array_element_indentation"
    );

    #[test]
    fn same_line_elements_ignored() {
        let source = b"x = [1, 2, 3]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, source);
        assert!(diags.is_empty());
    }

    #[test]
    fn align_brackets_style() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("align_brackets".into()),
            )]),
            ..CopConfig::default()
        };
        // Element at bracket_col + width (4 + 2 = 6), bracket at bracket_col (4) => good
        let src = b"x = [\n      1\n    ]\n";
        let diags = run_cop_full_with_config(&FirstArrayElementIndentation, src, config.clone());
        assert!(
            diags.is_empty(),
            "align_brackets should accept element at bracket_col + width: {:?}",
            diags
        );

        // Element indented normally (2 from line start) should be flagged
        let src2 = b"x = [\n  1\n]\n";
        let diags2 = run_cop_full_with_config(&FirstArrayElementIndentation, src2, config.clone());
        assert!(
            diags2.len() >= 1,
            "align_brackets should flag element not at bracket_col + width: {:?}",
            diags2
        );

        // Bracket not aligned with opening bracket should be flagged
        let src3 = b"x = [\n      1\n]\n";
        let diags3 = run_cop_full_with_config(&FirstArrayElementIndentation, src3, config);
        assert_eq!(
            diags3.len(),
            1,
            "align_brackets should flag bracket not at opening bracket column: {:?}",
            diags3
        );
    }

    #[test]
    fn special_inside_parentheses_method_call() {
        // Array argument with [ on same line as ( should use paren-relative indent
        // foo( is at col 3, so expected = 3 + 1 + 2 = 6
        let src = b"foo([\n      :bar,\n      :baz\n    ])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "array arg with [ on same line as ( should not be flagged"
        );
    }

    #[test]
    fn special_inside_parentheses_nested_call() {
        // expect(cli.run([ -- the ( of run( is at col 14, expected = 14 + 1 + 2 = 17
        let src =
            b"expect(cli.run([\n                 :a,\n                 :b\n               ]))\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "nested call array arg should use innermost paren"
        );
    }

    #[test]
    fn array_with_method_chain_uses_line_indent() {
        // [].join() -- array followed by .join() should use line-relative indent
        let src = b"expect(x).to eq([\n  'hello',\n  'world'\n].join(\"\\n\"))\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "array with .join chain should use line-relative indent"
        );
    }

    #[test]
    fn array_in_grouping_paren_uses_line_indent() {
        // (%i[...] + other) -- grouping paren, array followed by + operator
        let src = b"X = (%i[\n  a\n  b\n] + other).freeze\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "array in grouping paren with + operator should use line-relative indent"
        );
    }

    #[test]
    fn percent_i_array_inside_method_call_paren() {
        // %i[ inside eq() - should use paren-relative indent
        // eq( is at col 0-2, ( at col 2, so expected = 2 + 1 + 2 = 5
        let src = b"eq(%i[\n     :a,\n     :b\n   ])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "%i[ inside method call paren should use paren-relative indent: {:?}",
            diags
        );
    }

    #[test]
    fn percent_i_array_inside_method_call_paren_wrong_indent() {
        // %i[ inside eq() with wrong indent - should flag both element and bracket
        // eq( is at col 0-2, ( at col 2, so expected element = 2 + 1 + 2 = 5, but element is at col 2
        // Expected bracket = 2 + 1 = 3, but ] is at col 0
        let src = b"eq(%i[\n  :a,\n  :b\n])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert_eq!(
            diags.len(),
            2,
            "%i[ inside method call paren with wrong indent should flag element and bracket: {:?}",
            diags
        );
    }

    #[test]
    fn closing_bracket_wrong_indent_in_method_call() {
        // Mirrors the doorkeeper false negative: closing bracket at wrong indent
        // inside method call parens. eq( has ( at col 39.
        // indent_base = 39 + 1 = 40. Expected ] at col 40. Actual ] at col 4.
        // Also the first element at col 6 is wrong (expected 42).
        let src =
            b"    expect(validation_attributes).to eq(%i[\n      client_id\n      client\n    ])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        // Should flag both element (col 6 instead of 42) and bracket (col 4 instead of 40)
        assert_eq!(
            diags.len(),
            2,
            "should flag both element and bracket in method call: {:?}",
            diags
        );
        // Verify the bracket diagnostic
        let bracket_diag = diags
            .iter()
            .find(|d| d.message.contains("right bracket"))
            .unwrap();
        assert!(
            bracket_diag
                .message
                .contains("first position after the preceding left parenthesis"),
            "bracket message should reference left parenthesis: {}",
            bracket_diag.message
        );
    }

    #[test]
    fn closing_bracket_on_same_line_as_last_element_not_flagged() {
        // When ] is on the same line as the last element, don't check bracket indent
        let src = b"x = [\n  1,\n  2]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "bracket on same line as last element should not be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn closing_bracket_correct_indent_no_parens() {
        // ] at same indentation as the line with [ (indent_base = 0)
        let src = b"x = [\n  1,\n  2\n]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(
            diags.is_empty(),
            "bracket at line indent should not be flagged: {:?}",
            diags
        );
    }

    #[test]
    fn closing_bracket_wrong_indent_no_parens() {
        // ] at wrong indentation (should be at 0 but is at 2)
        let src = b"x = [\n  1,\n  2\n  ]\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert_eq!(
            diags.len(),
            1,
            "bracket at wrong indent should be flagged: {:?}",
            diags
        );
        assert!(
            diags[0].message.contains("right bracket"),
            "should be a bracket message: {}",
            diags[0].message
        );
    }
}
