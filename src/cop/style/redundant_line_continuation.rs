use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects redundant line continuations (`\` at end of line).
///
/// A line continuation is redundant when Ruby would naturally continue parsing
/// the expression without it. This includes:
/// - After operators and opening brackets (`,`, `.`, `(`, `[`, `{`, `+`, etc.)
/// - When the next line starts with `.` or `&.` (method chain continuation)
/// - After the `do` keyword (block start)
/// - After `class` or `module` keywords (definition start)
///
/// ## Remaining FN gap
/// The oracle corpus includes ~900 FN where `\` precedes `&&` or `||` on the
/// next line (e.g., `value \` + `&& other`). RuboCop 1.84.2 on Ruby 4.0 does
/// NOT flag these patterns either — its `redundant_line_continuation?` syntax
/// check now considers them required because of how the Prism parser handles
/// newline-separated boolean expressions. The oracle was likely generated with
/// an older RuboCop or parser version. Flagging these would risk FP regressions
/// against the current RuboCop baseline.
pub struct RedundantLineContinuation;

impl Cop for RedundantLineContinuation {
    fn name(&self) -> &'static str {
        "Style/RedundantLineContinuation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let lines: Vec<&[u8]> = source.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = trim_end(line);
            if !trimmed.ends_with(b"\\") {
                continue;
            }

            // Check the character before backslash is not another backslash (string escape)
            if trimmed.len() >= 2 && trimmed[trimmed.len() - 2] == b'\\' {
                continue;
            }

            // Compute the absolute offset of the backslash to check if it's in code
            let line_start = {
                let src = source.as_bytes();
                let mut offset = 0;
                let mut line_num = 0;
                for &b in src.iter() {
                    if line_num == i {
                        break;
                    }
                    offset += 1;
                    if b == b'\n' {
                        line_num += 1;
                    }
                }
                offset
            };
            let backslash_offset = line_start + trimmed.len() - 1;

            // Use code_map to verify the backslash is in a code region
            // (not inside a string, heredoc, or comment)
            if !code_map.is_code(backslash_offset) {
                continue;
            }

            let before_backslash = trim_end(&trimmed[..trimmed.len() - 1]);

            // Check if the continuation is redundant
            if is_redundant_continuation(before_backslash, i, &lines) {
                let col = trimmed.len() - 1;
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    col,
                    "Redundant line continuation.".to_string(),
                ));
            }
        }
    }
}

fn trim_end(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
        end -= 1;
    }
    &bytes[..end]
}

fn trim_start(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < bytes.len() && (bytes[start] == b' ' || bytes[start] == b'\t') {
        start += 1;
    }
    &bytes[start..]
}

fn is_redundant_continuation(before_backslash: &[u8], line_idx: usize, lines: &[&[u8]]) -> bool {
    let trimmed = trim_end(before_backslash);
    if trimmed.is_empty() {
        return false;
    }

    let last_byte = trimmed[trimmed.len() - 1];

    // After operators and opening brackets, continuation is redundant
    if matches!(
        last_byte,
        b',' | b'('
            | b'['
            | b'{'
            | b'+'
            | b'-'
            | b'*'
            | b'/'
            | b'|'
            | b'&'
            | b'.'
            | b'='
            | b'>'
            | b'<'
            | b'\\'
            | b':'
    ) {
        return true;
    }

    // After `do` keyword: block start, continuation is redundant
    if ends_with_keyword(trimmed, b"do") {
        return true;
    }

    // After `class` or `module` keyword followed by an identifier
    if line_has_keyword_def(trimmed) {
        return true;
    }

    // Check if the next line starts with `.` or `&.` (method chain continuation).
    // This makes the backslash redundant because Ruby naturally continues
    // the expression when the next line starts with a dot.
    // Exception: if there's a blank line between, the backslash may be required
    // to bridge the gap in a leading-dot method chain.
    if let Some(next_line) = lines.get(line_idx + 1) {
        let next_trimmed = trim_start(next_line);
        if next_trimmed.starts_with(b".") || next_trimmed.starts_with(b"&.") {
            // Check for blank line: if next line is blank, don't flag
            // (the blank line case is handled differently)
            if !next_trimmed.is_empty() {
                return true;
            }
        }
    }

    false
}

/// Check if the trimmed line ends with a specific keyword.
/// Ensures it's a whole word (preceded by whitespace or start of line).
fn ends_with_keyword(trimmed: &[u8], keyword: &[u8]) -> bool {
    if trimmed.len() < keyword.len() {
        return false;
    }
    let start = trimmed.len() - keyword.len();
    if &trimmed[start..] != keyword {
        return false;
    }
    // Must be at start of line or preceded by non-alphanumeric
    if start == 0 {
        return true;
    }
    let prev = trimmed[start - 1];
    !prev.is_ascii_alphanumeric() && prev != b'_'
}

/// Check if the line is a `class Foo` or `module Foo` definition.
/// The pattern is: keyword followed by a constant name.
fn line_has_keyword_def(trimmed: &[u8]) -> bool {
    let s = trim_start(trimmed);
    if s.starts_with(b"class ") && s.len() > 6 {
        // Ensure what follows is an identifier (not just `class << self`)
        let after = trim_start(&s[6..]);
        if !after.is_empty() && after[0].is_ascii_uppercase() {
            return true;
        }
    }
    if s.starts_with(b"module ") && s.len() > 7 {
        let after = trim_start(&s[7..]);
        if !after.is_empty() && after[0].is_ascii_uppercase() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantLineContinuation,
        "cops/style/redundant_line_continuation"
    );
}
