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
