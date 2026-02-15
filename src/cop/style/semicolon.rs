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
    ) -> Vec<Diagnostic> {
        let bytes = source.as_bytes();
        let lines: Vec<&[u8]> = source.lines().collect();
        let allow_separator = config.get_bool("AllowAsExpressionSeparator", false);
        let mut diagnostics = Vec::new();

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

        diagnostics
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
        || trimmed.starts_with(b"module ");
    starts_keyword && trimmed.ends_with(b"; end")
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Semicolon, "cops/style/semicolon");
}
