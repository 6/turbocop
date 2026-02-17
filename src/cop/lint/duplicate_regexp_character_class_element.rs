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
                // Find matching ]
                let start = i + 1;
                let mut j = start;
                // Handle negation
                if j < chars.len() && chars[j] == '^' {
                    j += 1;
                }
                // The first character after [ or [^ can be ] without closing
                if j < chars.len() && chars[j] == ']' {
                    j += 1;
                }

                while j < chars.len() {
                    if chars[j] == ']' && (j == 0 || chars[j - 1] != '\\') {
                        break;
                    }
                    j += 1;
                }

                if j < chars.len() {
                    // Extract the class content (between [ and ])
                    let class_content: String = chars[start..j].iter().collect();
                    // Check for duplicate single characters (not ranges)
                    let mut seen = std::collections::HashSet::new();
                    let class_chars: Vec<char> = class_content.chars().collect();
                    let mut k = 0;
                    // Handle ^ at the start
                    if k < class_chars.len() && class_chars[k] == '^' {
                        k += 1;
                    }
                    while k < class_chars.len() {
                        if class_chars[k] == '\\' && k + 1 < class_chars.len() {
                            // Escaped character, treat as a single entity
                            let escaped: String = class_chars[k..k + 2].iter().collect();
                            if !seen.insert(escaped.clone()) {
                                // Duplicate found - find the location in the source
                                // Calculate byte offset for this position
                                let char_offset: usize = chars[..start + k].iter().map(|c| c.len_utf8()).sum();
                                let byte_pos = content_start + char_offset;
                                if byte_pos < bytes.len() {
                                    let (line, column) = source.offset_to_line_col(byte_pos);
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        line,
                                        column,
                                        "Duplicate element inside regexp character class".to_string(),
                                    ));
                                }
                            }
                            k += 2;
                        } else if k + 2 < class_chars.len() && class_chars[k + 1] == '-' {
                            // Range like a-z, skip as a unit
                            let range: String = class_chars[k..k + 3].iter().collect();
                            if !seen.insert(range) {
                                let char_offset: usize = chars[..start + k].iter().map(|c| c.len_utf8()).sum();
                                let byte_pos = content_start + char_offset;
                                if byte_pos < bytes.len() {
                                    let (line, column) = source.offset_to_line_col(byte_pos);
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        line,
                                        column,
                                        "Duplicate element inside regexp character class".to_string(),
                                    ));
                                }
                            }
                            k += 3;
                        } else {
                            // Single character
                            let ch = class_chars[k].to_string();
                            if !seen.insert(ch.clone()) {
                                let char_offset: usize = chars[..start + k].iter().map(|c| c.len_utf8()).sum();
                                let byte_pos = content_start + char_offset;
                                if byte_pos < bytes.len() {
                                    let (line, column) = source.offset_to_line_col(byte_pos);
                                    diagnostics.push(self.diagnostic(
                                        source,
                                        line,
                                        column,
                                        "Duplicate element inside regexp character class".to_string(),
                                    ));
                                }
                            }
                            k += 1;
                        }
                    }
                }
                i = j + 1;
            } else {
                i += 1;
            }
        }

        diagnostics
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
