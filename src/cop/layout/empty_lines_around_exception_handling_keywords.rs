use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundExceptionHandlingKeywords;

const KEYWORDS: &[&[u8]] = &[b"rescue", b"ensure", b"else"];

impl Cop for EmptyLinesAroundExceptionHandlingKeywords {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundExceptionHandlingKeywords"
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut diagnostics = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let line_num = i + 1;
            let trimmed_start = match line.iter().position(|&b| b != b' ' && b != b'\t') {
                Some(p) => p,
                None => continue,
            };
            let content = &line[trimmed_start..];

            // Check if this line is a rescue/ensure/else keyword at the start of a line
            let matched_keyword = KEYWORDS.iter().find(|&&kw| {
                if content.starts_with(kw) {
                    let after = content.get(kw.len()..);
                    match after {
                        Some(rest) => rest.is_empty() || rest[0] == b' ' || rest[0] == b'\n' || rest[0] == b'\r',
                        None => true,
                    }
                } else {
                    false
                }
            });

            let keyword = match matched_keyword {
                Some(kw) => *kw,
                None => continue,
            };

            let kw_str = std::str::from_utf8(keyword).unwrap_or("rescue");

            // "else" is only relevant in rescue context — check if we're inside begin/def block
            // Simple heuristic: skip standalone else from if/unless by checking that indentation
            // suggests it's inside a rescue/begin block.

            // Check for empty line BEFORE the keyword
            if line_num >= 3 {
                let above_idx = i - 1; // 0-indexed
                if above_idx < lines.len() && util::is_blank_line(lines[above_idx]) {
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num - 1,
                        0,
                        format!("Extra empty line detected before the `{kw_str}`."),
                    ));
                }
            }

            // Check for empty line AFTER the keyword
            let below_idx = i + 1; // 0-indexed for line after
            if below_idx < lines.len() && util::is_blank_line(lines[below_idx]) {
                diagnostics.push(self.diagnostic(
                    source,
                    line_num + 1,
                    0,
                    format!("Extra empty line detected after the `{kw_str}`."),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAroundExceptionHandlingKeywords,
        "cops/layout/empty_lines_around_exception_handling_keywords"
    );
}
