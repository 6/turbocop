use crate::cop::shared::node_type::{
    INTERPOLATED_REGULAR_EXPRESSION_NODE, RANGE_NODE, REGULAR_EXPRESSION_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for mixed-case character ranges that include unintended characters.
/// For example, `('A'..'z')` includes `[`, `\`, `]`, `^`, `_`, `` ` ``.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=27.
///
/// FN:
/// - The original implementation only looked at Ruby `RangeNode`s like `('A'..'z')`.
///   Most corpus misses are regexp character classes such as `/[a-zA-z0-9]/`.
/// - Some remaining misses came from interpolated regexps where the unsafe range lives in a
///   literal segment around `#{...}`.
/// - Unicode property escapes like `\p{InLatin_Extended-A}` contain `-` inside the property
///   name; those must be skipped as atomic escapes instead of scanned as `d-A`.
///
/// ## Corpus investigation (2026-03-22)
///
/// RuboCop also flags single-character ranges whose bounds cross between an ASCII letter range
/// and a non-letter, such as `('0'..'z')`, `(' '..'z')`, `('['..'z')`, and
/// `("\x21".."\x5A")`. The previous implementation only flagged lowercase-vs-uppercase letter
/// pairs, so these range-node cases were missed even though RuboCop treats them as unsafe.
///
/// The first repair attempt reused that broader rule for regexp character classes too, which
/// regressed corpus repos on accepted patterns like `/[_-a]/` and `/[A-_]/`. Keep regexp ranges
/// on the narrower "upper-vs-lower letter bucket" rule, and apply the broader comparison only to
/// Ruby `RangeNode`s.
pub struct MixedCaseRange;

const MSG: &str = "Ranges from upper to lower case ASCII letters may include unintended characters. Instead of `A-z` (which also includes several symbols) specify each range individually: `A-Za-z` and individually specify any symbols.";

impl Cop for MixedCaseRange {
    fn name(&self) -> &'static str {
        "Lint/MixedCaseRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
            RANGE_NODE,
            REGULAR_EXPRESSION_NODE,
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
        if let Some(range) = node.as_range_node() {
            diagnostics.extend(self.check_range(source, range));
            return;
        }

        if let Some(regexp) = node.as_regular_expression_node() {
            self.check_regexp(source, regexp, diagnostics);
            return;
        }

        if let Some(regexp) = node.as_interpolated_regular_expression_node() {
            self.check_interpolated_regexp(source, regexp, diagnostics);
        }
    }
}

impl MixedCaseRange {
    fn check_range(
        &self,
        source: &SourceFile,
        range: ruby_prism::RangeNode<'_>,
    ) -> Vec<Diagnostic> {
        let left = match range.left() {
            Some(l) => l,
            None => return Vec::new(),
        };
        let right = match range.right() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Both must be string literals
        let left_str = match left.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };
        let right_str = match right.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let left_val = left_str.unescaped();
        let right_val = right_str.unescaped();

        // Must be single characters
        if left_val.len() != 1 || right_val.len() != 1 {
            return Vec::new();
        }

        let left_char = left_val[0] as char;
        let right_char = right_val[0] as char;

        if is_unsafe_char_range(left_char, right_char) {
            let loc = range.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(source, line, column, MSG.to_string())];
        }

        Vec::new()
    }

    fn check_regexp(
        &self,
        source: &SourceFile,
        regexp: ruby_prism::RegularExpressionNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Ok(content) = std::str::from_utf8(regexp.content_loc().as_slice()) else {
            return;
        };

        let mut offsets = Vec::new();
        let mut offset = regexp.content_loc().start_offset();
        for ch in content.chars() {
            offsets.push(Some(offset));
            offset += ch.len_utf8();
        }

        self.check_regexp_chars(
            source,
            &content.chars().collect::<Vec<_>>(),
            &offsets,
            diagnostics,
        );
    }

    fn check_interpolated_regexp(
        &self,
        source: &SourceFile,
        regexp: ruby_prism::InterpolatedRegularExpressionNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
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

            chars.push('\0');
            offsets.push(None);
        }

        self.check_regexp_chars(source, &chars, &offsets, diagnostics);
    }

    fn check_regexp_chars(
        &self,
        source: &SourceFile,
        chars: &[char],
        offsets: &[Option<usize>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        debug_assert_eq!(chars.len(), offsets.len());

        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '[' && (i == 0 || chars[i - 1] != '\\') {
                let Some(class_end) = find_char_class_end(chars, i) else {
                    i += 1;
                    continue;
                };
                self.check_regexp_class(source, chars, offsets, i + 1, class_end, diagnostics);
                i = class_end + 1;
            } else {
                i += 1;
            }
        }
    }

    fn check_regexp_class(
        &self,
        source: &SourceFile,
        chars: &[char],
        offsets: &[Option<usize>],
        start: usize,
        end: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut i = start;
        if i < end && chars[i] == '^' {
            i += 1;
        }

        while i < end {
            if chars[i] == '[' {
                if let Some(nested_end) = find_char_class_end(chars, i) {
                    i = nested_end + 1;
                    continue;
                }
                i += 1;
                continue;
            }

            if chars[i] == '\\' {
                i += escape_sequence_len(chars, i);
                continue;
            }

            if i + 2 < end && chars[i + 1] == '-' && chars[i + 2] != ']' {
                let range_end = chars[i + 2];
                if range_end == '\\' || range_end == '[' {
                    i += 1;
                    continue;
                }

                if is_unsafe_regexp_range(chars[i], range_end) {
                    if let Some(abs_offset) = offsets.get(i).copied().flatten() {
                        let (line, column) = source.offset_to_line_col(abs_offset);
                        diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
                    }
                }

                i += 3;
            } else {
                i += 1;
            }
        }
    }
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
        if chars[i] == '\\' {
            i += escape_sequence_len(chars, i);
        } else if chars[i] == '[' {
            if i + 1 < chars.len() && chars[i + 1] == ':' {
                i += 2;
                while i + 1 < chars.len() {
                    if chars[i] == ':' && chars[i + 1] == ']' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            } else if let Some(nested_end) = find_char_class_end(chars, i) {
                i = nested_end + 1;
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
    if start + 1 >= chars.len() {
        return 1;
    }

    match chars[start + 1] {
        'x' => {
            let mut count = 2;
            let mut i = start + 2;
            while i < chars.len() && count < 4 && chars[i].is_ascii_hexdigit() {
                count += 1;
                i += 1;
            }
            count
        }
        'u' => {
            if start + 2 < chars.len() && chars[start + 2] == '{' {
                let mut i = start + 3;
                while i < chars.len() && chars[i] != '}' {
                    i += 1;
                }
                if i < chars.len() {
                    i + 1 - start
                } else {
                    i - start
                }
            } else {
                let mut count = 2;
                let mut i = start + 2;
                while i < chars.len() && count < 6 && chars[i].is_ascii_hexdigit() {
                    count += 1;
                    i += 1;
                }
                count
            }
        }
        '0'..='7' => {
            let mut count = 2;
            let mut i = start + 2;
            while i < chars.len() && count < 4 && matches!(chars[i], '0'..='7') {
                count += 1;
                i += 1;
            }
            count
        }
        'c' => {
            if start + 2 < chars.len() {
                3
            } else {
                2
            }
        }
        'p' | 'P' => {
            if start + 2 < chars.len() && chars[start + 2] == '{' {
                let mut i = start + 3;
                while i < chars.len() && chars[i] != '}' {
                    i += 1;
                }
                if i < chars.len() {
                    i + 1 - start
                } else {
                    i - start
                }
            } else {
                2
            }
        }
        _ => 2,
    }
}

fn char_range(c: char) -> Option<u8> {
    if c.is_ascii_lowercase() {
        Some(0) // a-z
    } else if c.is_ascii_uppercase() {
        Some(1) // A-Z
    } else {
        None
    }
}

fn is_unsafe_char_range(start: char, end: char) -> bool {
    char_range(start) != char_range(end)
}

fn is_unsafe_regexp_range(start: char, end: char) -> bool {
    let start_range = char_range(start);
    let end_range = char_range(end);

    match (start_range, end_range) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MixedCaseRange, "cops/lint/mixed_case_range");
}
