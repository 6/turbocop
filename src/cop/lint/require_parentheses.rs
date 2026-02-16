use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct RequireParentheses;

impl Cop for RequireParentheses {
    fn name(&self) -> &'static str {
        "Lint/RequireParentheses"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    /// This cop detects ambiguous method calls where a predicate method (ending
    /// with `?`) is called without parentheses and the last argument contains
    /// `&&` or `||`. Because Prism resolves precedence correctly, we can't
    /// detect this from the AST alone — we use a source-based approach instead.
    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut byte_offset: usize = 0;

        for (line_idx, line) in source.lines().enumerate() {
            let line_num = line_idx + 1;
            let first_non_ws = line.iter().position(|&b| b != b' ' && b != b'\t');
            let indent = first_non_ws.unwrap_or(line.len());
            let trimmed = &line[indent..];

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with(b"#") {
                byte_offset += line.len() + 1; // +1 for newline
                continue;
            }

            // Look for `word? ` (predicate method followed by space, no paren)
            if find_predicate_without_parens(trimmed) {
                let pred_abs = byte_offset + indent;
                if code_map.is_code(pred_abs) {
                    // Check if there's && or || later on the same line (in code context)
                    if has_logical_operator_in_code(trimmed, byte_offset + indent, code_map) {
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            indent,
                            "Use parentheses in the method call to avoid confusion about precedence."
                                .to_string(),
                        ));
                    }
                }
            }

            byte_offset += line.len() + 1; // +1 for newline
        }

        diagnostics
    }
}

/// Check if the trimmed line contains a predicate method call without parentheses:
/// `name? arg` (not `name?(arg)`). Excludes ternary operator `x ? y : z` where
/// there's a space before the `?`.
fn find_predicate_without_parens(trimmed: &[u8]) -> bool {
    let mut i = 0;
    while i < trimmed.len() {
        if trimmed[i] == b'?' && i > 0 {
            let prev = trimmed[i - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                // Check next char is space (not `?(` which has parens)
                if i + 1 < trimmed.len() && trimmed[i + 1] == b' ' {
                    // Distinguish from ternary: in a ternary, there's a space before the `?`
                    // In a predicate method, `?` is attached to the method name: `foo?`
                    // Find the start of the identifier before `?`
                    let mut start = i - 1;
                    while start > 0
                        && (trimmed[start - 1].is_ascii_alphanumeric()
                            || trimmed[start - 1] == b'_')
                    {
                        start -= 1;
                    }
                    // If the char before the identifier is a space, it could be ternary
                    // e.g., `bar ? x : y` — `bar` preceded by space, `?` followed by space
                    // For predicate: `foo? bar` or `.foo? bar` or `if foo? bar`
                    // The key: in ternary, there's also a `:` later. But simpler check:
                    // if preceded by `.` it's definitely a method call
                    // if the identifier looks like a method name (contains `?` already embedded)
                    // For now: require that the char before the identifier is `.` OR
                    // the identifier itself is the first thing on the line (possibly after `if`/`unless` etc.)
                    if start > 0 && trimmed[start - 1] == b'.' {
                        return true; // obj.method? arg
                    }
                    // Also match bare predicate calls like `is? arg` or `include? arg`
                    // Exclude if there's a `:` after the `?` part (ternary pattern)
                    let after_q = &trimmed[i + 1..];
                    if !after_q.contains(&b':') {
                        return true;
                    }
                    // Has `:` — could be ternary. Check if the `:` is a ternary else
                    // (not a symbol literal): look for ` : ` pattern
                    let has_ternary_colon = after_q
                        .windows(3)
                        .any(|w| w == b" : ");
                    if !has_ternary_colon {
                        return true; // `:` is probably inside a symbol like `:jan`
                    }
                    // Has ternary colon — skip this `?` (likely ternary operator)
                }
            }
        }
        i += 1;
    }
    false
}

/// Check if the rest of the line contains `&&` or `||` in code context.
fn has_logical_operator_in_code(bytes: &[u8], base_offset: usize, code_map: &CodeMap) -> bool {
    for i in 0..bytes.len().saturating_sub(1) {
        if (bytes[i] == b'&' && bytes[i + 1] == b'&')
            || (bytes[i] == b'|' && bytes[i + 1] == b'|')
        {
            if code_map.is_code(base_offset + i) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RequireParentheses, "cops/lint/require_parentheses");
}
