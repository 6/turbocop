use crate::cop::shared::node_type::{
    INTERPOLATED_REGULAR_EXPRESSION_NODE, REGULAR_EXPRESSION_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Investigation (2026-03-03)
///
/// Found 4 FPs: `[0]` and `[ ]` single-element character classes. These ARE
/// genuinely redundant — the cop detection is correct. RuboCop doesn't flag
/// them because the project's style gem likely disables this cop. Not a cop
/// logic bug — this is a config resolution issue.
///
/// ## FN investigation (2026-03-27)
///
/// Corpus FNs came from three detection gaps in the original byte-based scanner:
/// - Interpolated regexps like `/#{tag}[\s]/` were ignored entirely because the cop
///   only visited `RegularExpressionNode`.
/// - UTF-8 literals such as `[髙]` and `[埼]` were counted byte-by-byte, so one
///   codepoint looked like multiple elements and was skipped.
/// - Nested sets like `[ef-g[h]]` were treated as opaque outer classes, so the
///   inner redundant `[h]` was never revisited.
///
/// One reported corpus FN (`ammar/regexp_parser` `spec/scanner/sets_spec.rb:97`)
/// reproduces as detected in isolation and is a config/context mismatch rather than
/// a cop logic bug. This fix stays in cop logic only.
pub struct RedundantRegexpCharacterClass;

const REQUIRES_ESCAPE_OUTSIDE_CHAR_CLASS_CHARS: [char; 10] =
    ['.', '*', '+', '?', '{', '}', '(', ')', '|', '$'];

impl Cop for RedundantRegexpCharacterClass {
    fn name(&self) -> &'static str {
        "Style/RedundantRegexpCharacterClass"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            REGULAR_EXPRESSION_NODE,
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(regexp) = node.as_regular_expression_node() {
            let Ok(content) = std::str::from_utf8(regexp.content_loc().as_slice()) else {
                return;
            };
            let (chars, offsets) = chars_with_offsets(content, regexp.content_loc().start_offset());
            check_regexp_fragment(
                self,
                source,
                &chars,
                &offsets,
                regexp.is_extended(),
                regexp.is_extended(),
                diagnostics,
            );
            return;
        }

        if let Some(regexp) = node.as_interpolated_regular_expression_node() {
            let mut chars = Vec::new();
            let mut offsets = Vec::new();

            for part in regexp.parts().iter() {
                if let Some(string) = part.as_string_node() {
                    let Ok(content) = std::str::from_utf8(string.content_loc().as_slice()) else {
                        return;
                    };
                    append_chars_with_offsets(
                        &mut chars,
                        &mut offsets,
                        content,
                        string.content_loc().start_offset(),
                    );
                    continue;
                }

                chars.push('\0');
                offsets.push(None);
            }

            let extended = is_extended_regex(regexp.closing_loc().as_slice());
            check_regexp_fragment(
                self,
                source,
                &chars,
                &offsets,
                extended,
                extended,
                diagnostics,
            );
        }
    }
}

fn chars_with_offsets(content: &str, start_offset: usize) -> (Vec<char>, Vec<Option<usize>>) {
    let mut chars = Vec::with_capacity(content.chars().count());
    let mut offsets = Vec::with_capacity(chars.capacity());
    append_chars_with_offsets(&mut chars, &mut offsets, content, start_offset);
    (chars, offsets)
}

fn append_chars_with_offsets(
    chars: &mut Vec<char>,
    offsets: &mut Vec<Option<usize>>,
    content: &str,
    start_offset: usize,
) {
    let mut offset = start_offset;
    for ch in content.chars() {
        chars.push(ch);
        offsets.push(Some(offset));
        offset += ch.len_utf8();
    }
}

fn is_extended_regex(closing_loc: &[u8]) -> bool {
    closing_loc.contains(&b'x')
}

fn check_regexp_fragment(
    cop: &RedundantRegexpCharacterClass,
    source: &SourceFile,
    chars: &[char],
    offsets: &[Option<usize>],
    extended_mode: bool,
    skip_comments: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\0' {
            i += 1;
            continue;
        }

        if skip_comments && chars[i] == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if is_unescaped_open_bracket(chars, i) {
            if let Some(end) = find_char_class_end(chars, i) {
                check_character_class(
                    cop,
                    source,
                    chars,
                    offsets,
                    i..end,
                    extended_mode,
                    diagnostics,
                );
                i = end + 1;
                continue;
            }
        }

        if chars[i] == '\\' && i + 1 < chars.len() {
            i += escape_sequence_len(chars, i);
        } else {
            i += 1;
        }
    }
}

fn check_character_class(
    cop: &RedundantRegexpCharacterClass,
    source: &SourceFile,
    chars: &[char],
    offsets: &[Option<usize>],
    class_range: std::ops::Range<usize>,
    extended_mode: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let open = class_range.start;
    let close = class_range.end;
    if open + 1 < close {
        check_regexp_fragment(
            cop,
            source,
            &chars[open + 1..close],
            &offsets[open + 1..close],
            extended_mode,
            false,
            diagnostics,
        );
    }

    let class_content = &chars[open + 1..close];
    let Some((char_class, replacement)) =
        redundant_single_element_character_class(class_content, extended_mode)
    else {
        return;
    };

    let Some(byte_pos) = offsets.get(open).copied().flatten() else {
        return;
    };
    let (line, column) = source.offset_to_line_col(byte_pos);
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!(
            "Redundant single-element character class, `{char_class}` can be replaced with `{replacement}`."
        ),
    ));
}

fn redundant_single_element_character_class(
    class_content: &[char],
    extended_mode: bool,
) -> Option<(String, String)> {
    if class_content.is_empty() || class_content[0] == '^' {
        return None;
    }

    let (kind, consumed) = parse_class_element(class_content, 0)?;
    if consumed != class_content.len() {
        return None;
    }

    let element = match kind {
        ElementKind::Candidate(element) => element,
        ElementKind::NonRedundantSingle => return None,
    };

    if whitespace_in_free_space_mode(extended_mode, &element)
        || backslash_b(&element)
        || octal_requiring_char_class(&element)
        || requires_escape_outside_char_class(&element)
    {
        return None;
    }

    let replacement = if element == "#" {
        r"\#".to_string()
    } else {
        element.clone()
    };

    Some((format!("[{element}]"), replacement))
}

enum ElementKind {
    Candidate(String),
    NonRedundantSingle,
}

fn parse_class_element(class_content: &[char], start: usize) -> Option<(ElementKind, usize)> {
    if start >= class_content.len() {
        return None;
    }

    if class_content[start] == '\0' {
        return None;
    }

    if class_content[start] == '[' {
        if let Some(end) = parse_posix_class(class_content, start) {
            return Some((ElementKind::NonRedundantSingle, end));
        }

        let end = find_char_class_end(class_content, start)?;
        return Some((ElementKind::NonRedundantSingle, end + 1));
    }

    if class_content[start] == '\\' && start + 1 < class_content.len() {
        let esc_len = escape_sequence_len(class_content, start);
        let element: String = class_content[start..start + esc_len].iter().collect();
        let kind = if multiple_codepoints_escape(&element) {
            ElementKind::NonRedundantSingle
        } else {
            ElementKind::Candidate(element)
        };
        return Some((kind, start + esc_len));
    }

    Some((
        ElementKind::Candidate(class_content[start].to_string()),
        start + 1,
    ))
}

fn parse_posix_class(chars: &[char], start: usize) -> Option<usize> {
    if chars.get(start) != Some(&'[') || chars.get(start + 1) != Some(&':') {
        return None;
    }

    let mut i = start + 2;
    while i + 1 < chars.len() {
        if chars[i] == ':' && chars[i + 1] == ']' {
            return Some(i + 2);
        }
        i += 1;
    }
    None
}

fn whitespace_in_free_space_mode(extended_mode: bool, element: &str) -> bool {
    extended_mode
        && element.chars().count() == 1
        && element.chars().next().is_some_and(char::is_whitespace)
}

fn backslash_b(element: &str) -> bool {
    element == r"\b"
}

fn octal_requiring_char_class(element: &str) -> bool {
    let bytes = element.as_bytes();
    bytes.len() == 2 && bytes[0] == b'\\' && (b'1'..=b'7').contains(&bytes[1])
}

fn requires_escape_outside_char_class(element: &str) -> bool {
    let mut chars = element.chars();
    let Some(ch) = chars.next() else {
        return false;
    };
    chars.next().is_none() && REQUIRES_ESCAPE_OUTSIDE_CHAR_CLASS_CHARS.contains(&ch)
}

fn multiple_codepoints_escape(element: &str) -> bool {
    if !(element.starts_with(r"\u{") && element.ends_with('}')) {
        return false;
    }

    element[3..element.len() - 1]
        .split_whitespace()
        .nth(1)
        .is_some()
}

fn is_unescaped_open_bracket(chars: &[char], pos: usize) -> bool {
    if chars[pos] != '[' {
        return false;
    }

    let mut backslash_count = 0;
    let mut i = pos;
    while i > 0 {
        i -= 1;
        if chars[i] == '\\' {
            backslash_count += 1;
        } else {
            break;
        }
    }

    backslash_count % 2 == 0
}

fn find_char_class_end(chars: &[char], open: usize) -> Option<usize> {
    let mut i = open + 1;
    if i < chars.len() && chars[i] == '^' {
        i += 1;
    }
    if i < chars.len() && chars[i] == ']' {
        i += 1;
    }

    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            i += escape_sequence_len(chars, i);
        } else if chars[i] == '[' {
            if let Some(end) = parse_posix_class(chars, i) {
                i = end;
            } else if let Some(end) = find_char_class_end(chars, i) {
                i = end + 1;
            } else {
                i += 1;
            }
        } else if chars[i] == ']' {
            return Some(i);
        } else {
            i += 1;
        }
    }

    None
}

fn escape_sequence_len(chars: &[char], start: usize) -> usize {
    let len = chars.len();
    if start + 1 >= len {
        return 1;
    }

    match chars[start + 1] {
        'x' => {
            let mut count = 2;
            let mut i = start + 2;
            while i < len && count < 4 && chars[i].is_ascii_hexdigit() {
                count += 1;
                i += 1;
            }
            count
        }
        'u' => {
            if start + 2 < len && chars[start + 2] == '{' {
                let mut i = start + 3;
                while i < len && chars[i] != '}' {
                    i += 1;
                }
                if i < len { i + 1 - start } else { i - start }
            } else {
                let mut count = 2;
                let mut i = start + 2;
                while i < len && count < 6 && chars[i].is_ascii_hexdigit() {
                    count += 1;
                    i += 1;
                }
                count
            }
        }
        'p' | 'P' => {
            if start + 2 < len && chars[start + 2] == '{' {
                let mut i = start + 3;
                while i < len && chars[i] != '}' {
                    i += 1;
                }
                if i < len { i + 1 - start } else { i - start }
            } else {
                2
            }
        }
        'c' => {
            if start + 2 < len {
                3
            } else {
                2
            }
        }
        '0'..='7' => {
            let mut count = 2;
            let mut i = start + 2;
            while i < len && count < 4 && ('0'..='7').contains(&chars[i]) {
                count += 1;
                i += 1;
            }
            count
        }
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantRegexpCharacterClass,
        "cops/style/redundant_regexp_character_class"
    );
}
