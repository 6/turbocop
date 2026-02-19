use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct FirstArrayElementIndentation;

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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let array_node = match node.as_array_node() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let opening_loc = match array_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let elements: Vec<_> = array_node.elements().iter().collect();
        if elements.is_empty() {
            return Vec::new();
        }

        let first_element = &elements[0];

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let first_loc = first_element.location();
        let (elem_line, elem_col) = source.offset_to_line_col(first_loc.start_offset());

        // Skip if first element is on same line as opening bracket
        if elem_line == open_line {
            return Vec::new();
        }

        let style = config.get_str("EnforcedStyle", "special_inside_parentheses");
        let width = config.get_usize("IndentationWidth", 2);

        // Get the indentation of the line where `[` appears
        let open_line_bytes = source.lines().nth(open_line - 1).unwrap_or(b"");
        let open_line_indent = indentation_of(open_line_bytes);
        let (_, open_col) = source.offset_to_line_col(opening_loc.start_offset());

        let expected = match style {
            "consistent" => open_line_indent + width,
            "align_brackets" => open_col,
            _ => {
                // "special_inside_parentheses" (default):
                // If the `[` is on the same line as a method call's `(`,
                // and the array is a direct argument (not part of a chain
                // like `[...].join()` or `[...] + other`), indent relative
                // to the position after `(`.
                // Otherwise, indent relative to line start.
                let closing_end = array_node
                    .closing_loc()
                    .map(|loc| loc.end_offset())
                    .unwrap_or(0);
                if let Some(paren_col) = find_left_paren_on_line(open_line_bytes, open_col) {
                    if is_direct_argument(source.as_bytes(), closing_end) {
                        paren_col + 1 + width
                    } else {
                        open_line_indent + width
                    }
                } else {
                    open_line_indent + width
                }
            }
        };

        if elem_col != expected {
            let base = elem_col.saturating_sub(open_line_indent);
            return vec![self.diagnostic(
                source,
                elem_line,
                elem_col,
                format!(
                    "Use {} (not {}) spaces for indentation of the first element.",
                    width, base
                ),
            )];
        }

        Vec::new()
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
        use std::collections::HashMap;
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([
                ("EnforcedStyle".into(), serde_yml::Value::String("align_brackets".into())),
            ]),
            ..CopConfig::default()
        };
        // Element aligned with opening bracket column
        let src = b"x = [\n    1\n]\n";
        let diags = run_cop_full_with_config(&FirstArrayElementIndentation, src, config.clone());
        assert!(diags.is_empty(), "align_brackets should accept element at bracket column");

        // Element indented normally (2 from line start) should be flagged
        let src2 = b"x = [\n  1\n]\n";
        let diags2 = run_cop_full_with_config(&FirstArrayElementIndentation, src2, config);
        assert_eq!(diags2.len(), 1, "align_brackets should flag element not at bracket column");
    }

    #[test]
    fn special_inside_parentheses_method_call() {
        // Array argument with [ on same line as ( should use paren-relative indent
        // foo( is at col 3, so expected = 3 + 1 + 2 = 6
        let src = b"foo([\n      :bar,\n      :baz\n    ])\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(diags.is_empty(), "array arg with [ on same line as ( should not be flagged");
    }

    #[test]
    fn special_inside_parentheses_nested_call() {
        // expect(cli.run([ -- the ( of run( is at col 14, expected = 14 + 1 + 2 = 17
        let src = b"expect(cli.run([\n                 :a,\n                 :b\n               ]))\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(diags.is_empty(), "nested call array arg should use innermost paren");
    }

    #[test]
    fn array_with_method_chain_uses_line_indent() {
        // [].join() -- array followed by .join() should use line-relative indent
        let src = b"expect(x).to eq([\n  'hello',\n  'world'\n].join(\"\\n\"))\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(diags.is_empty(), "array with .join chain should use line-relative indent");
    }

    #[test]
    fn array_in_grouping_paren_uses_line_indent() {
        // (%i[...] + other) -- grouping paren, array followed by + operator
        let src = b"X = (%i[\n  a\n  b\n] + other).freeze\n";
        let diags = run_cop_full(&FirstArrayElementIndentation, src);
        assert!(diags.is_empty(), "array in grouping paren with + operator should use line-relative indent");
    }
}
