use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AlignRightLetBrace;

impl Cop for AlignRightLetBrace {
    fn name(&self) -> &'static str {
        "RSpec/AlignRightLetBrace"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        let mut i = 0;
        while i < lines.len() {
            let mut group: Vec<(usize, usize)> = Vec::new(); // (line_num, closing_brace_col)

            while i < lines.len() {
                if let Some(brace_col) = single_line_let_close_brace_col(lines[i]) {
                    group.push((i + 1, brace_col));
                    i += 1;
                } else if is_blank_or_comment(lines[i]) && !group.is_empty() {
                    break;
                } else if !group.is_empty() {
                    if is_multiline_let(lines[i]) {
                        break;
                    }
                    break;
                } else {
                    i += 1;
                    break;
                }
            }

            if group.len() >= 2 {
                let max_col = group.iter().map(|(_, c)| *c).max().unwrap_or(0);
                for &(line_num, brace_col) in &group {
                    if brace_col != max_col {
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            brace_col,
                            "Align right let brace.".to_string(),
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

fn single_line_let_close_brace_col(line: &[u8]) -> Option<usize> {
    let s = std::str::from_utf8(line).ok()?;
    let trimmed = s.trim_start();
    if !trimmed.starts_with("let(") && !trimmed.starts_with("let!(") {
        return None;
    }

    // Must have opening and closing brace on same line
    let open_brace = s.find('{')?;
    let close_brace = s.rfind('}')?;

    if close_brace <= open_brace {
        return None;
    }

    Some(close_brace)
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
    crate::cop_fixture_tests!(AlignRightLetBrace, "cops/rspec/align_right_let_brace");
}
