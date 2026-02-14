use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AlignLeftLetBrace;

impl Cop for AlignLeftLetBrace {
    fn name(&self) -> &'static str {
        "RSpec/AlignLeftLetBrace"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        // Find groups of consecutive `let` declarations and check brace alignment
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        // Find groups of consecutive single-line let declarations
        let mut i = 0;
        while i < lines.len() {
            let mut group: Vec<(usize, usize)> = Vec::new(); // (line_num, brace_col)

            while i < lines.len() {
                if let Some(brace_col) = single_line_let_brace_col(lines[i]) {
                    group.push((i + 1, brace_col));
                    i += 1;
                } else if is_blank_or_comment(lines[i]) && !group.is_empty() {
                    // Blank/comment lines break groups
                    break;
                } else if !group.is_empty() {
                    // Non-let line after some lets - also check multiline let which breaks group
                    if is_multiline_let(lines[i]) {
                        // skip multiline lets
                        break;
                    }
                    break;
                } else {
                    i += 1;
                    break;
                }
            }

            // If blank line, skip and continue group (but groups should be separated)
            if group.len() >= 2 {
                let max_col = group.iter().map(|(_, c)| *c).max().unwrap_or(0);
                for &(line_num, brace_col) in &group {
                    if brace_col != max_col {
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            brace_col,
                            "Align left let brace.".to_string(),
                        ));
                    }
                }
            }

            if group.is_empty() {
                // Already incremented
            }
        }

        diagnostics
    }
}

fn single_line_let_brace_col(line: &[u8]) -> Option<usize> {
    let s = std::str::from_utf8(line).ok()?;
    let trimmed = s.trim_start();
    if !trimmed.starts_with("let(") && !trimmed.starts_with("let!(") {
        return None;
    }

    // Find `) {` pattern - single-line let with braces
    let paren_close = s.find(')')?;
    let after_paren = &s[paren_close + 1..];
    let trimmed_after = after_paren.trim_start();

    if !trimmed_after.starts_with('{') {
        return None;
    }

    // Must also have closing brace on same line
    let brace_open = paren_close + 1 + (after_paren.len() - trimmed_after.len());
    if !s[brace_open..].contains('}') {
        return None;
    }

    Some(brace_open)
}

fn is_blank_or_comment(line: &[u8]) -> bool {
    let s = std::str::from_utf8(line).unwrap_or("");
    let trimmed = s.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

fn is_multiline_let(line: &[u8]) -> bool {
    let s = std::str::from_utf8(line).unwrap_or("");
    let trimmed = s.trim_start();
    (trimmed.starts_with("let(") || trimmed.starts_with("let!("))
        && trimmed.ends_with('{')
        && !trimmed.contains('}')
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AlignLeftLetBrace, "cops/rspec/align_left_let_brace");
}
