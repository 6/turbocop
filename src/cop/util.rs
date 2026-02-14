use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

/// Count body lines between start and end offsets (exclusive of keyword lines).
/// Skips blank lines. Optionally skips comment-only lines.
pub fn count_body_lines(
    source: &SourceFile,
    start_offset: usize,
    end_offset: usize,
    count_comments: bool,
) -> usize {
    let (start_line, _) = source.offset_to_line_col(start_offset);
    let (end_line, _) = source.offset_to_line_col(end_offset);

    // Count lines between (exclusive of def/end lines)
    let lines: Vec<&[u8]> = source.lines().collect();
    let mut count = 0;

    // Lines between start_line and end_line (exclusive)
    // start_line and end_line are 1-indexed
    for line_num in (start_line + 1)..end_line {
        if line_num > lines.len() {
            break;
        }
        let line = lines[line_num - 1]; // convert to 0-indexed
        let trimmed = trim_bytes(line);

        // Skip blank lines
        if trimmed.is_empty() {
            continue;
        }

        // Optionally skip comment-only lines
        if !count_comments && trimmed.starts_with(b"#") {
            continue;
        }

        count += 1;
    }

    count
}

fn trim_bytes(b: &[u8]) -> &[u8] {
    let start = b.iter().position(|&c| c != b' ' && c != b'\t' && c != b'\r');
    match start {
        Some(s) => {
            let end = b.iter().rposition(|&c| c != b' ' && c != b'\t' && c != b'\r').unwrap();
            &b[s..=end]
        }
        None => &[],
    }
}

/// Check if a name is snake_case (lowercase + digits + underscores, not starting with uppercase).
pub fn is_snake_case(name: &[u8]) -> bool {
    if name.is_empty() {
        return true;
    }
    // Allow leading underscores (e.g., _foo)
    // Must not contain uppercase letters
    for &b in name {
        if b.is_ascii_uppercase() {
            return false;
        }
        if !(b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
            // Allow ? and ! at end for Ruby method names
            if b == b'?' || b == b'!' || b == b'=' {
                continue;
            }
            return false;
        }
    }
    true
}

/// Check if a name is SCREAMING_SNAKE_CASE (uppercase + digits + underscores).
pub fn is_screaming_snake_case(name: &[u8]) -> bool {
    if name.is_empty() {
        return true;
    }
    for &b in name {
        if b.is_ascii_lowercase() {
            return false;
        }
        if !(b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') {
            return false;
        }
    }
    true
}

/// Check if a name is CamelCase (starts uppercase, no underscores).
pub fn is_camel_case(name: &[u8]) -> bool {
    if name.is_empty() {
        return false;
    }
    if !name[0].is_ascii_uppercase() {
        return false;
    }
    // Allow digits, no underscores (except leading _ is not CamelCase)
    for &b in &name[1..] {
        if b == b'_' {
            return false;
        }
        if !(b.is_ascii_alphanumeric()) {
            return false;
        }
    }
    true
}

/// Check if all bytes in a name are ASCII.
pub fn is_ascii_name(name: &[u8]) -> bool {
    name.iter().all(|b| b.is_ascii())
}

/// Info about a 2-method chain: `receiver.inner_method(...).outer_method(...)`.
pub struct MethodChain<'a> {
    /// The inner CallNode (the receiver of the outer call).
    pub inner_call: ruby_prism::CallNode<'a>,
    /// The method name of the inner call.
    pub inner_method: &'a [u8],
    /// The method name of the outer call.
    pub outer_method: &'a [u8],
}

/// Extract a 2-method chain from a node.
///
/// If `node` is a CallNode `x.outer()` whose receiver is also a CallNode `y.inner()`,
/// returns `Some(MethodChain { inner_call, inner_method, outer_method })`.
pub fn as_method_chain<'a>(node: &ruby_prism::Node<'a>) -> Option<MethodChain<'a>> {
    let outer_call = node.as_call_node()?;
    let outer_method = outer_call.name().as_slice();
    let receiver = outer_call.receiver()?;
    let inner_call = receiver.as_call_node()?;
    let inner_method = inner_call.name().as_slice();
    Some(MethodChain {
        inner_call,
        inner_method,
        outer_method,
    })
}

/// Check if the line above a node's start offset is a comment line.
pub fn preceding_comment_line(source: &SourceFile, node_start_offset: usize) -> bool {
    let (node_line, _) = source.offset_to_line_col(node_start_offset);
    if node_line <= 1 {
        return false;
    }
    let lines: Vec<&[u8]> = source.lines().collect();
    let prev_line = lines.get(node_line - 2); // node_line is 1-indexed, prev is node_line-1, 0-indexed = node_line-2
    match prev_line {
        Some(line) => {
            let trimmed = trim_bytes(line);
            trimmed.starts_with(b"#")
        }
        None => false,
    }
}

/// Check if a node spans exactly one line in the source.
pub fn node_on_single_line(source: &SourceFile, loc: &ruby_prism::Location<'_>) -> bool {
    let (start_line, _) = source.offset_to_line_col(loc.start_offset());
    let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
    let (end_line, _) = source.offset_to_line_col(end_offset);
    start_line == end_line
}

/// Compute the expected indentation column for body statements
/// given the keyword's column and the configured width.
pub fn expected_indent_for_body(keyword_column: usize, width: usize) -> usize {
    keyword_column + width
}

/// Get the line content at a given 1-indexed line number.
pub fn line_at(source: &SourceFile, line_number: usize) -> Option<&[u8]> {
    source.lines().nth(line_number - 1)
}

/// Get the indentation (number of leading spaces) for a byte slice.
pub fn indentation_of(line: &[u8]) -> usize {
    line.iter().take_while(|&&b| b == b' ').count()
}

/// Check if there is a trailing comma between last_element_end and closing_start.
pub fn has_trailing_comma(
    source_bytes: &[u8],
    last_element_end: usize,
    closing_start: usize,
) -> bool {
    if last_element_end >= closing_start || closing_start > source_bytes.len() {
        return false;
    }
    source_bytes[last_element_end..closing_start]
        .iter()
        .any(|&b| b == b',')
}

// ── Shared cop logic helpers ──────────────────────────────────────────

/// Check if a line is blank (only whitespace).
pub fn is_blank_line(line: &[u8]) -> bool {
    line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r')
}

/// Check for extra empty lines at the beginning/end of a body.
/// Used by EmptyLinesAround{Class,Module,Method,Block}Body.
pub fn check_empty_lines_around_body(
    cop_name: &str,
    source: &SourceFile,
    keyword_offset: usize,
    end_offset: usize,
    body_kind: &str,
) -> Vec<Diagnostic> {
    let (keyword_line, _) = source.offset_to_line_col(keyword_offset);
    let (end_line, _) = source.offset_to_line_col(end_offset);

    if keyword_line == end_line {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();

    // Check for blank line after keyword
    let after_keyword = keyword_line + 1;
    if let Some(line) = line_at(source, after_keyword) {
        if is_blank_line(line) && after_keyword < end_line {
            diagnostics.push(Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line: after_keyword, column: 0 },
                severity: Severity::Convention,
                cop_name: cop_name.to_string(),
                message: format!("Extra empty line detected at {body_kind} body beginning."),
            });
        }
    }

    // Check for blank line before end
    if end_line > 1 {
        let before_end = end_line - 1;
        if before_end > keyword_line {
            if let Some(line) = line_at(source, before_end) {
                if is_blank_line(line) {
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location { line: before_end, column: 0 },
                        severity: Severity::Convention,
                        cop_name: cop_name.to_string(),
                        message: format!("Extra empty line detected at {body_kind} body end."),
                    });
                }
            }
        }
    }

    diagnostics
}

/// Check that `end` is aligned with the opening keyword.
/// Used by DefEndAlignment, EndAlignment, ElseAlignment.
pub fn check_keyword_end_alignment(
    cop_name: &str,
    source: &SourceFile,
    keyword_name: &str,
    keyword_offset: usize,
    end_offset: usize,
) -> Vec<Diagnostic> {
    let (_, kw_col) = source.offset_to_line_col(keyword_offset);
    let (end_line, end_col) = source.offset_to_line_col(end_offset);

    if end_col != kw_col {
        return vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line: end_line, column: end_col },
            severity: Severity::Convention,
            cop_name: cop_name.to_string(),
            message: format!("Align `end` with `{keyword_name}`."),
        }];
    }

    Vec::new()
}

/// Check first element indentation relative to an opening delimiter.
/// Used by FirstArgument/Array/HashElementIndentation.
pub fn check_first_element_indentation(
    cop_name: &str,
    source: &SourceFile,
    width: usize,
    opening_offset: usize,
    first_element_offset: usize,
) -> Vec<Diagnostic> {
    let (open_line, _) = source.offset_to_line_col(opening_offset);
    let (elem_line, elem_col) = source.offset_to_line_col(first_element_offset);

    // Skip if on same line as opener
    if elem_line == open_line {
        return Vec::new();
    }

    let open_line_bytes = source.lines().nth(open_line - 1).unwrap_or(b"");
    let open_indent = indentation_of(open_line_bytes);
    let expected = open_indent + width;

    if elem_col != expected {
        return vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line: elem_line, column: elem_col },
            severity: Severity::Convention,
            cop_name: cop_name.to_string(),
            message: format!(
                "Use {} (not {}) spaces for indentation of the first element.",
                width,
                elem_col.saturating_sub(open_indent)
            ),
        }];
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_snake_case() {
        assert!(is_snake_case(b"foo_bar"));
        assert!(is_snake_case(b"foo"));
        assert!(is_snake_case(b"_foo"));
        assert!(is_snake_case(b"foo_bar_baz"));
        assert!(is_snake_case(b"foo123"));
        assert!(is_snake_case(b"valid?"));
        assert!(is_snake_case(b"save!"));
        assert!(!is_snake_case(b"FooBar"));
        assert!(!is_snake_case(b"fooBar"));
        assert!(!is_snake_case(b"FOO"));
    }

    #[test]
    fn test_is_screaming_snake_case() {
        assert!(is_screaming_snake_case(b"FOO_BAR"));
        assert!(is_screaming_snake_case(b"FOO"));
        assert!(is_screaming_snake_case(b"MAX_SIZE"));
        assert!(is_screaming_snake_case(b"V2"));
        assert!(!is_screaming_snake_case(b"foo_bar"));
        assert!(!is_screaming_snake_case(b"FooBar"));
        assert!(!is_screaming_snake_case(b"Foo"));
    }

    #[test]
    fn test_is_camel_case() {
        assert!(is_camel_case(b"FooBar"));
        assert!(is_camel_case(b"Foo"));
        assert!(is_camel_case(b"FooBarBaz"));
        assert!(is_camel_case(b"Foo123"));
        assert!(!is_camel_case(b"foo_bar"));
        assert!(!is_camel_case(b"FOO_BAR"));
        assert!(!is_camel_case(b"Foo_Bar"));
        assert!(!is_camel_case(b""));
    }

    #[test]
    fn test_is_ascii_name() {
        assert!(is_ascii_name(b"foo_bar"));
        assert!(is_ascii_name(b"FooBar"));
        assert!(!is_ascii_name("café".as_bytes()));
        assert!(!is_ascii_name("naïve".as_bytes()));
    }

    #[test]
    fn test_has_trailing_comma() {
        let src = b"[1, 2, 3,]";
        // '3' ends at byte 8, ']' at byte 9
        assert!(has_trailing_comma(src, 8, 9));
        let src2 = b"[1, 2, 3]";
        // '3' ends at byte 8, ']' at byte 8 — no room for comma
        assert!(!has_trailing_comma(src2, 8, 8));
    }

    #[test]
    fn test_count_body_lines() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"def foo\n  x = 1\n  y = 2\n  # comment\n\n  z = 3\nend\n".to_vec(),
        );
        // def starts at offset 0 (line 1), end starts at offset 45 (line 7)
        // Lines 2-6: "  x = 1", "  y = 2", "  # comment", "", "  z = 3"
        // Without comments: 3 lines (x, y, z)
        assert_eq!(count_body_lines(&source, 0, 45, false), 3);
        // With comments: 4 lines (x, y, #comment, z)
        assert_eq!(count_body_lines(&source, 0, 45, true), 4);
    }
}
