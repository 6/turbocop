use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct Semicolon;

impl Cop for Semicolon {
    fn name(&self) -> &'static str {
        "Style/Semicolon"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let bytes = source.as_bytes();
        let lines: Vec<&[u8]> = source.lines().collect();
        let allow_separator = config.get_bool("AllowAsExpressionSeparator", false);

        for (i, &byte) in bytes.iter().enumerate() {
            if byte != b';' || !code_map.is_code(i) {
                continue;
            }

            let (line, column) = source.offset_to_line_col(i);

            // Get the line content (1-indexed)
            let line_bytes = match lines.get(line - 1) {
                Some(l) => l,
                None => continue,
            };
            let trimmed = trim_bytes(line_bytes);

            // Skip single-line def/class/module bodies (e.g., `def show; end`).
            // RuboCop handles these via Style/EmptyMethod and Style/SingleLineMethods.
            if is_single_line_body(trimmed) {
                continue;
            }

            // Skip semicolons that are part of embedded `def foo; end` patterns
            // (e.g., inside a block: `{ def foo; end }`)
            if is_embedded_single_line_body(line_bytes, column) {
                continue;
            }

            // AllowAsExpressionSeparator: skip semicolons used between expressions
            if allow_separator && column + 1 < line_bytes.len() {
                let after = trim_bytes(&line_bytes[column + 1..]);
                if !after.is_empty() && !after.starts_with(b"#") {
                    continue;
                }
            }

            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not use semicolons to terminate expressions.".to_string(),
            ));
        }

    }
}

fn trim_bytes(b: &[u8]) -> &[u8] {
    let start = b.iter().position(|&c| c != b' ' && c != b'\t').unwrap_or(b.len());
    let end = b.iter().rposition(|&c| c != b' ' && c != b'\t' && c != b'\n' && c != b'\r').map_or(start, |e| e + 1);
    if start >= end { &[] } else { &b[start..end] }
}

/// Check if a trimmed line is a single-line body that RuboCop doesn't flag.
/// Patterns: `def foo; end`, `def foo; something; end`,
/// `class Foo < Bar; end`, `module Foo; end`
fn is_single_line_body(trimmed: &[u8]) -> bool {
    let starts_keyword = trimmed.starts_with(b"def ")
        || trimmed.starts_with(b"class ")
        || trimmed.starts_with(b"module ")
        || trimmed.starts_with(b"while ")
        || trimmed.starts_with(b"until ")
        || trimmed.starts_with(b"begin");
    starts_keyword && trimmed.ends_with(b"; end")
}

/// Check if a semicolon at a given column is part of a `def/class/module ... ; end`
/// pattern embedded within a larger line (e.g., inside a block).
/// RuboCop doesn't flag these because its token-based detection doesn't find them.
fn is_embedded_single_line_body(line_bytes: &[u8], semicolon_col: usize) -> bool {
    // Look backwards from the semicolon for a keyword
    let before = &line_bytes[..semicolon_col];
    let after = &line_bytes[semicolon_col + 1..];

    // Check if there's a `def ` before the semicolon (possibly with other stuff before)
    let has_def = find_keyword_before(before, b"def ");
    let has_class = find_keyword_before(before, b"class ");
    let has_module = find_keyword_before(before, b"module ");

    if !has_def && !has_class && !has_module {
        return false;
    }

    // Check if after the semicolon there's `end` or `...; end` pattern
    let trimmed_after = trim_bytes_start(after);
    if trimmed_after.starts_with(b"end")
        && (trimmed_after.len() == 3
            || !trimmed_after[3].is_ascii_alphanumeric() && trimmed_after[3] != b'_')
    {
        return true;
    }
    // Also check for `something; end` after
    if let Some(next_semi) = after.iter().position(|&b| b == b';') {
        let after_next = trim_bytes_start(&after[next_semi + 1..]);
        if after_next.starts_with(b"end")
            && (after_next.len() == 3
                || !after_next[3].is_ascii_alphanumeric() && after_next[3] != b'_')
        {
            return true;
        }
    }
    false
}

fn find_keyword_before(before: &[u8], keyword: &[u8]) -> bool {
    // Search for the keyword preceded by a non-alphanumeric character (or start of string)
    if before.len() < keyword.len() {
        return false;
    }
    for i in 0..=before.len() - keyword.len() {
        if &before[i..i + keyword.len()] == keyword {
            if i == 0 || !before[i - 1].is_ascii_alphanumeric() && before[i - 1] != b'_' {
                return true;
            }
        }
    }
    false
}

fn trim_bytes_start(b: &[u8]) -> &[u8] {
    let start = b.iter().position(|&c| c != b' ' && c != b'\t').unwrap_or(b.len());
    &b[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Semicolon, "cops/style/semicolon");
}
