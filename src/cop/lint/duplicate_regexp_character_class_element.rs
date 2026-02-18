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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let regexp = match node.as_regular_expression_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let content = regexp.unescaped();
        let content_str = match std::str::from_utf8(&content) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let mut diagnostics = Vec::new();
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
                    let has_intersection = class_content.windows(2).any(|w| w[0] == '&' && w[1] == '&');
                    if !has_intersection {
                        check_class_for_duplicates(
                            self,
                            source,
                            &chars,
                            class_content,
                            start,
                            content_start,
                            bytes,
                            &mut diagnostics,
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

        diagnostics
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
            j += 2; // skip escaped char
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
                emit_duplicate(cop, source, all_chars, class_start_in_all + k, content_start, bytes, diagnostics);
            }
            k = p;
        } else if class_content[k] == '[' {
            // Nested character class (e.g. [a-z[0-9]]) — skip as a single entity
            let nested_chars: Vec<char> = class_content[k..].to_vec();
            if let Some(end) = find_char_class_end(&nested_chars, 0) {
                let entity: String = nested_chars[..=end].iter().collect();
                if !seen.insert(entity) {
                    emit_duplicate(cop, source, all_chars, class_start_in_all + k, content_start, bytes, diagnostics);
                }
                k += end + 1;
            } else {
                k += 1;
            }
        } else if class_content[k] == '\\' && k + 1 < class_content.len() {
            // Handle \p{...} and \P{...} Unicode property escapes as single entity
            let next = class_content[k + 1];
            if (next == 'p' || next == 'P') && k + 2 < class_content.len() && class_content[k + 2] == '{' {
                // Find closing }
                let mut p = k + 3;
                while p < class_content.len() && class_content[p] != '}' {
                    p += 1;
                }
                if p < class_content.len() {
                    p += 1; // include the }
                }
                let entity: String = class_content[k..p].iter().collect();
                if !seen.insert(entity) {
                    emit_duplicate(cop, source, all_chars, class_start_in_all + k, content_start, bytes, diagnostics);
                }
                k = p;
            } else {
                // Regular escaped character: \s, \d, \n, etc.
                let escaped: String = class_content[k..k + 2].iter().collect();
                if !seen.insert(escaped) {
                    emit_duplicate(cop, source, all_chars, class_start_in_all + k, content_start, bytes, diagnostics);
                }
                k += 2;
            }
        } else if k + 2 < class_content.len()
            && class_content[k + 1] == '-'
            && class_content[k + 2] != ']'
        {
            // Range like a-z, skip as a unit
            let range: String = class_content[k..k + 3].iter().collect();
            if !seen.insert(range) {
                emit_duplicate(cop, source, all_chars, class_start_in_all + k, content_start, bytes, diagnostics);
            }
            k += 3;
        } else {
            // Single character
            let ch = class_content[k].to_string();
            if !seen.insert(ch) {
                emit_duplicate(cop, source, all_chars, class_start_in_all + k, content_start, bytes, diagnostics);
            }
            k += 1;
        }
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
