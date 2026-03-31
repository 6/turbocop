use crate::cop::node_type::{INTERPOLATED_REGULAR_EXPRESSION_NODE, REGULAR_EXPRESSION_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for duplicate elements in Regexp character classes.
/// For example, `/[xyx]/` has a duplicate `x`.
///
/// ## Corpus investigation (2026-03-14)
///
/// FP root causes:
/// - The escape check `chars[i-1] != '\\'` failed on `\\[` (escaped backslash + literal bracket).
///   `\\` is an escaped backslash, so the `[` after it is NOT escaped. Fixed by counting
///   consecutive preceding backslashes: odd count means the bracket is escaped, even means it's not.
/// - Using `regexp.unescaped()` instead of raw source bytes caused mismatches between parsed
///   content and source offsets. Switched to `content_loc().as_slice()` for raw source.
///
/// FN root causes:
/// - Interpolated regexes (`InterpolatedRegularExpressionNode`) were not handled at all.
///   Added support by extracting string parts from interpolated regex nodes.
///
/// ## Extended mode FP fix (2026-03-14)
///
/// In extended mode (`/x`), `#` starts a comment until end of line. The cop was treating
/// comment text as regex content, finding false "duplicate" character class elements from
/// bracket characters in comments. Fixed by detecting the `/x` flag and skipping from `#`
/// to end of line in `check_regexp_content` (outside character classes only, since `#` is
/// literal inside `[...]`).
///
/// ## Corpus investigation update (2026-03-15)
///
/// Corpus oracle reported the remaining FN=1 on degenerate ranges such as
/// `[A-Aa-z0-9]`. RuboCop treats `A-A` as duplicating the endpoint element and
/// reports the second `A`, not the range as a unique entity.
///
/// ## Corpus investigation update (2026-03-29)
///
/// The final FN was an interpolated character class from `riscv-unified-db`:
/// `/^([[#{Regexp.escape(exclude_item)}(?:,.*?)?]])\s*$/`.
/// The scanner bailed out when interpolation placeholders appeared inside the
/// class, so it missed repeated literal `?` elements around the interpolation.
/// Fixed by skipping interpolation placeholders instead of abandoning the whole
/// class.
///
/// ## Corpus investigation update (2026-03-31)
///
/// The remaining FP cluster came from nested character classes such as
/// `[[a-c][x-z][0-2]]` and `[[[:alpha:]][[:blank:]]]`. The scanner flattened
/// nested `[` and `]` into ordinary characters, so it reported duplicate
/// brackets instead of treating nested sets as grouped elements. Fixed by
/// recognizing nested `[...]` groups inside a class, tracking the whole nested
/// set as one element for duplicate detection, and recursively scanning the
/// nested set body so real duplicates like `[[a][a]]` still report.
pub struct DuplicateRegexpCharacterClassElement;

impl Cop for DuplicateRegexpCharacterClassElement {
    fn name(&self) -> &'static str {
        "Lint/DuplicateRegexpCharacterClassElement"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
            let content_slice = regexp.content_loc().as_slice();
            let Ok(content_str) = std::str::from_utf8(content_slice) else {
                return;
            };
            let content_start = regexp.content_loc().start_offset();
            let chars: Vec<char> = content_str.chars().collect();
            let mut offsets = Vec::with_capacity(chars.len());
            let mut offset = content_start;
            for ch in &chars {
                offsets.push(Some(offset));
                offset += ch.len_utf8();
            }
            let extended = regexp.is_extended();
            check_regexp_content(self, source, &chars, &offsets, extended, diagnostics);
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
                    let mut offset = string.content_loc().start_offset();
                    for ch in content.chars() {
                        chars.push(ch);
                        offsets.push(Some(offset));
                        offset += ch.len_utf8();
                    }
                    continue;
                }

                // Non-string part (interpolation) — insert placeholder
                chars.push('\0');
                offsets.push(None);
            }

            let extended = is_extended_regex(regexp.closing_loc().as_slice());
            check_regexp_content(self, source, &chars, &offsets, extended, diagnostics);
        }
    }
}

/// Check if the closing location of a regex contains the `x` flag (extended mode).
fn is_extended_regex(closing_loc: &[u8]) -> bool {
    closing_loc.contains(&b'x')
}

/// Check whether the character at `pos` is an unescaped `[`.
/// Counts consecutive backslashes before `pos`: odd = bracket is escaped, even = not escaped.
fn is_unescaped_open_bracket(chars: &[char], pos: usize) -> bool {
    if chars[pos] != '[' {
        return false;
    }
    let mut backslash_count = 0;
    let mut p = pos;
    while p > 0 {
        p -= 1;
        if chars[p] == '\\' {
            backslash_count += 1;
        } else {
            break;
        }
    }
    // Even number of preceding backslashes means the bracket is NOT escaped
    backslash_count % 2 == 0
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

/// Top-level scan: find character classes in the regex and check each for duplicates.
fn check_regexp_content(
    cop: &DuplicateRegexpCharacterClassElement,
    source: &SourceFile,
    chars: &[char],
    offsets: &[Option<usize>],
    extended: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut i = 0;
    while i < chars.len() {
        // Skip null placeholders (interpolation boundaries)
        if chars[i] == '\0' {
            i += 1;
            continue;
        }
        // In extended mode, # starts a comment until end of line (outside character classes)
        if extended && chars[i] == '#' {
            // Skip to end of line
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if is_unescaped_open_bracket(chars, i) {
            // Find matching ] (handling nested [...], POSIX classes, escapes)
            let end = find_char_class_end(chars, i);
            if let Some(j) = end {
                // Extract content between [ and ]
                let start = i + 1;
                let class_content = &chars[start..j];

                // Skip character classes that use && (intersection) — too complex
                // to analyze for duplicates (matches RuboCop behavior).
                let has_intersection = class_content.windows(2).any(|w| w[0] == '&' && w[1] == '&');
                if !has_intersection {
                    check_class_for_duplicates(
                        cop,
                        source,
                        class_content,
                        &offsets[start..j],
                        diagnostics,
                    );
                }
                i = j + 1;
            } else {
                i += 1;
            }
        } else if chars[i] == '\\' && i + 1 < chars.len() {
            i += escape_sequence_len(chars, i);
        } else {
            i += 1;
        }
    }
}

/// Check a character class body for duplicate elements, emitting diagnostics.
fn check_class_for_duplicates(
    cop: &DuplicateRegexpCharacterClassElement,
    source: &SourceFile,
    class_content: &[char],
    class_offsets: &[Option<usize>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = std::collections::HashSet::new();
    let mut k = 0;
    // Handle ^ at the start
    if k < class_content.len() && class_content[k] == '^' {
        k += 1;
    }
    while k < class_content.len() {
        // Skip null placeholders (interpolation boundaries)
        if class_content[k] == '\0' {
            // Ignore the interpolated segment itself, but keep scanning the
            // surrounding literal elements in this character class.
            k += 1;
            continue;
        }
        if class_content[k] == '[' {
            // POSIX character classes like [:digit:] and [:alpha:] are
            // single grouped elements within a character class.
            if k + 1 < class_content.len() && class_content[k + 1] == ':' {
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
                    emit_duplicate(cop, source, class_offsets, k, diagnostics);
                }
                k = p;
                continue;
            }

            if let Some(nested_end) = find_char_class_end(class_content, k) {
                let nested_set: String = class_content[k..=nested_end].iter().collect();
                if !seen.insert(nested_set) {
                    emit_duplicate(cop, source, class_offsets, k, diagnostics);
                }

                if nested_end > k + 1 {
                    check_class_for_duplicates(
                        cop,
                        source,
                        &class_content[k + 1..nested_end],
                        &class_offsets[k + 1..nested_end],
                        diagnostics,
                    );
                }

                k = nested_end + 1;
                continue;
            }

            // Unmatched `[` in the slice: keep the existing literal fallback.
            let ch = class_content[k].to_string();
            if !seen.insert(ch) {
                emit_duplicate(cop, source, class_offsets, k, diagnostics);
            }
            k += 1;
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
                let range_end: String = class_content
                    [range_end_start..range_end_start + range_end_len]
                    .iter()
                    .collect();
                if entity == range_end {
                    seen.insert(entity);
                    emit_duplicate(cop, source, class_offsets, range_end_start, diagnostics);
                    k = range_end_start + range_end_len;
                    continue;
                }
                let range_str: String = class_content[k..range_end_start + range_end_len]
                    .iter()
                    .collect();
                if !seen.insert(range_str) {
                    emit_duplicate(cop, source, class_offsets, k, diagnostics);
                }
                k = range_end_start + range_end_len;
            } else {
                if !seen.insert(entity) {
                    emit_duplicate(cop, source, class_offsets, k, diagnostics);
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
            let start = class_content[k].to_string();
            let end: String = class_content[range_end_start..range_end_start + range_end_len]
                .iter()
                .collect();
            if start == end {
                seen.insert(start);
                emit_duplicate(cop, source, class_offsets, range_end_start, diagnostics);
                k = range_end_start + range_end_len;
                continue;
            }
            let range: String = class_content[k..range_end_start + range_end_len]
                .iter()
                .collect();
            if !seen.insert(range) {
                emit_duplicate(cop, source, class_offsets, k, diagnostics);
            }
            k = range_end_start + range_end_len;
        } else {
            // Single character
            let ch = class_content[k].to_string();
            if !seen.insert(ch) {
                emit_duplicate(cop, source, class_offsets, k, diagnostics);
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
    class_offsets: &[Option<usize>],
    k: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(byte_pos) = class_offsets.get(k).copied().flatten() {
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
