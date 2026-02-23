use crate::cop::node_type::REGULAR_EXPRESSION_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for duplicate elements in Regexp character classes.
/// For example, `/[xyx]/` has a duplicate `x`.
pub struct DuplicateRegexpCharacterClassElement;

impl Cop for DuplicateRegexpCharacterClassElement {
    fn name(&self) -> &'static str {
        "Lint/DuplicateRegexpCharacterClassElement"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[REGULAR_EXPRESSION_NODE]
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
        let regexp = match node.as_regular_expression_node() {
            Some(r) => r,
            None => return,
        };

        let content = regexp.unescaped();
        let content_str = match std::str::from_utf8(content) {
            Ok(s) => s,
            Err(_) => return,
        };

        let bytes = source.as_bytes();
        let content_loc = regexp.content_loc();
        let content_start = content_loc.start_offset();

        // Simple character class analysis: find [...] blocks and check for duplicates
        let mut i = 0;
        let chars: Vec<char> = content_str.chars().collect();
        while i < chars.len() {
            if chars[i] == '[' && (i == 0 || chars[i - 1] != '\\') {
                // Find matching ] (handling nested [...], POSIX classes, escapes)
                let end = find_char_class_end(&chars, i);
                if let Some(j) = end {
                    // Extract content between [ and ]
                    let start = i + 1;
                    let class_content = &chars[start..j];

                    // Skip character classes that use && (intersection) — too complex
                    // to analyze for duplicates (matches RuboCop behavior).
                    let has_intersection =
                        class_content.windows(2).any(|w| w[0] == '&' && w[1] == '&');
                    if !has_intersection {
                        check_class_for_duplicates(
                            self,
                            source,
                            &chars,
                            class_content,
                            start,
                            content_start,
                            bytes,
                            diagnostics,
                        );
                    }
                    i = j + 1;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }
}

/// Find the closing `]` for a character class starting at `chars[pos]` == `[`.
/// Returns `Some(index_of_closing_bracket)` or `None` if not found.
/// Handles nested `[...]`, POSIX `[:...:]`, and escape sequences.
fn find_char_class_end(chars: &[char], open: usize) -> Option<usize> {
    let mut j = open + 1;
    // Handle negation
    if j < chars.len() && chars[j] == '^' {
        j += 1;
    }
    // The first character after [ or [^ can be ] without closing
    if j < chars.len() && chars[j] == ']' {
        j += 1;
    }

    while j < chars.len() {
        if chars[j] == '\\' && j + 1 < chars.len() {
            j += escape_sequence_len(chars, j);
        } else if chars[j] == '[' {
            if j + 1 < chars.len() && chars[j + 1] == ':' {
                // POSIX character class like [:digit:] — skip to :]
                j += 2;
                while j + 1 < chars.len() {
                    if chars[j] == ':' && chars[j + 1] == ']' {
                        j += 2;
                        break;
                    }
                    j += 1;
                }
            } else {
                // Nested character class — recurse to find its end
                if let Some(nested_end) = find_char_class_end(chars, j) {
                    j = nested_end + 1;
                } else {
                    j += 1;
                }
            }
        } else if chars[j] == ']' {
            return Some(j);
        } else {
            j += 1;
        }
    }
    None
}

/// Check a character class body for duplicate elements, emitting diagnostics.
#[allow(clippy::too_many_arguments)] // internal helper with tightly-coupled params
fn check_class_for_duplicates(
    cop: &DuplicateRegexpCharacterClassElement,
    source: &SourceFile,
    all_chars: &[char],
    class_content: &[char],
    class_start_in_all: usize, // index into `all_chars` where class body starts
    content_start: usize,      // byte offset of regex content in source
    bytes: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = std::collections::HashSet::new();
    let mut k = 0;
    // Handle ^ at the start
    if k < class_content.len() && class_content[k] == '^' {
        k += 1;
    }
    while k < class_content.len() {
        // Skip POSIX character classes like [:digit:], [:alpha:], etc.
        if class_content[k] == '[' && k + 1 < class_content.len() && class_content[k + 1] == ':' {
            let mut p = k + 2;
            while p + 1 < class_content.len() {
                if class_content[p] == ':' && class_content[p + 1] == ']' {
                    p += 2;
                    break;
                }
                p += 1;
            }
            let posix_class: String = class_content[k..p].iter().collect();
            if !seen.insert(posix_class) {
                emit_duplicate(
                    cop,
                    source,
                    all_chars,
                    class_start_in_all + k,
                    content_start,
                    bytes,
                    diagnostics,
                );
            }
            k = p;
        } else if class_content[k] == '[' {
            // Nested character class (e.g. [a-z[0-9]]) — skip as a single entity
            let nested_chars: Vec<char> = class_content[k..].to_vec();
            if let Some(end) = find_char_class_end(&nested_chars, 0) {
                let entity: String = nested_chars[..=end].iter().collect();
                if !seen.insert(entity) {
                    emit_duplicate(
                        cop,
                        source,
                        all_chars,
                        class_start_in_all + k,
                        content_start,
                        bytes,
                        diagnostics,
                    );
                }
                k += end + 1;
            } else {
                k += 1;
            }
        } else if class_content[k] == '\\' && k + 1 < class_content.len() {
            let esc_len = escape_sequence_len(class_content, k);
            let entity: String = class_content[k..k + esc_len].iter().collect();

            // Check if this escape is followed by `-` forming a range
            let after_esc = k + esc_len;
            if after_esc + 1 < class_content.len()
                && class_content[after_esc] == '-'
                && class_content[after_esc + 1] != ']'
            {
                // Range where the start is an escape sequence (e.g. \x00-\x1F)
                let range_end_start = after_esc + 1;
                let range_end_len = if class_content[range_end_start] == '\\'
                    && range_end_start + 1 < class_content.len()
                {
                    escape_sequence_len(class_content, range_end_start)
                } else {
                    1
                };
                let range_str: String = class_content[k..range_end_start + range_end_len]
                    .iter()
                    .collect();
                if !seen.insert(range_str) {
                    emit_duplicate(
                        cop,
                        source,
                        all_chars,
                        class_start_in_all + k,
                        content_start,
                        bytes,
                        diagnostics,
                    );
                }
                k = range_end_start + range_end_len;
            } else {
                if !seen.insert(entity) {
                    emit_duplicate(
                        cop,
                        source,
                        all_chars,
                        class_start_in_all + k,
                        content_start,
                        bytes,
                        diagnostics,
                    );
                }
                k += esc_len;
            }
        } else if k + 2 < class_content.len()
            && class_content[k + 1] == '-'
            && class_content[k + 2] != ']'
        {
            // Range like a-z — the end might be an escape sequence
            let range_end_start = k + 2;
            let range_end_len = if class_content[range_end_start] == '\\'
                && range_end_start + 1 < class_content.len()
            {
                escape_sequence_len(class_content, range_end_start)
            } else {
                1
            };
            let range: String = class_content[k..range_end_start + range_end_len]
                .iter()
                .collect();
            if !seen.insert(range) {
                emit_duplicate(
                    cop,
                    source,
                    all_chars,
                    class_start_in_all + k,
                    content_start,
                    bytes,
                    diagnostics,
                );
            }
            k = range_end_start + range_end_len;
        } else {
            // Single character
            let ch = class_content[k].to_string();
            if !seen.insert(ch) {
                emit_duplicate(
                    cop,
                    source,
                    all_chars,
                    class_start_in_all + k,
                    content_start,
                    bytes,
                    diagnostics,
                );
            }
            k += 1;
        }
    }
}

/// Calculate the length of an escape sequence starting at `chars[start]` == `\`.
/// Handles: \xHH (4), \uHHHH (6), \u{...} (variable), \p{...}/\P{...} (variable),
/// \cX (3), \C-X (4), \M-X (4), \M-\C-X (6), octal \0nn (up to 4), and simple 2-char escapes.
fn escape_sequence_len(chars: &[char], start: usize) -> usize {
    let len = chars.len();
    if start + 1 >= len {
        return 1; // lone backslash at end
    }
    let next = chars[start + 1];
    match next {
        'x' => {
            // \xHH — up to 2 hex digits
            let mut count = 2; // \x
            let mut i = start + 2;
            while i < len && count < 4 && chars[i].is_ascii_hexdigit() {
                count += 1;
                i += 1;
            }
            count
        }
        'u' => {
            if start + 2 < len && chars[start + 2] == '{' {
                // \u{HHHH} — variable length
                let mut p = start + 3;
                while p < len && chars[p] != '}' {
                    p += 1;
                }
                if p < len {
                    p + 1 - start // include closing }
                } else {
                    p - start
                }
            } else {
                // \uHHHH — exactly 4 hex digits
                let mut count = 2; // \u
                let mut i = start + 2;
                while i < len && count < 6 && chars[i].is_ascii_hexdigit() {
                    count += 1;
                    i += 1;
                }
                count
            }
        }
        'p' | 'P' => {
            // \p{Name} or \P{Name}
            if start + 2 < len && chars[start + 2] == '{' {
                let mut p = start + 3;
                while p < len && chars[p] != '}' {
                    p += 1;
                }
                if p < len { p + 1 - start } else { p - start }
            } else {
                2
            }
        }
        'c' => {
            // \cX — control character
            if start + 2 < len { 3 } else { 2 }
        }
        '0'..='7' => {
            // Octal escape: \0, \00, \000, \1, \12, \123, etc.
            let mut count = 2; // \ + first digit
            let mut i = start + 2;
            while i < len && count < 4 && chars[i] >= '0' && chars[i] <= '7' {
                count += 1;
                i += 1;
            }
            count
        }
        _ => 2, // Simple 2-char escape: \n, \t, \s, \d, \w, etc.
    }
}

fn emit_duplicate(
    cop: &DuplicateRegexpCharacterClassElement,
    source: &SourceFile,
    all_chars: &[char],
    pos_in_all: usize,
    content_start: usize,
    bytes: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let char_offset: usize = all_chars[..pos_in_all].iter().map(|c| c.len_utf8()).sum();
    let byte_pos = content_start + char_offset;
    if byte_pos < bytes.len() {
        let (line, column) = source.offset_to_line_col(byte_pos);
        diagnostics.push(cop.diagnostic(
            source,
            line,
            column,
            "Duplicate element inside regexp character class".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DuplicateRegexpCharacterClassElement,
        "cops/lint/duplicate_regexp_character_class_element"
    );
}
