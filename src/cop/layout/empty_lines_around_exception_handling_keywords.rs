use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundExceptionHandlingKeywords;

const KEYWORDS: &[&[u8]] = &[b"rescue", b"ensure", b"else"];

impl Cop for EmptyLinesAroundExceptionHandlingKeywords {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundExceptionHandlingKeywords"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut byte_offset: usize = 0;

        for (i, line) in lines.iter().enumerate() {
            let line_len = line.len() + 1; // +1 for newline
            let line_num = i + 1;
            let trimmed_start = match line.iter().position(|&b| b != b' ' && b != b'\t') {
                Some(p) => p,
                None => {
                    byte_offset += line_len;
                    continue;
                }
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
                None => {
                    byte_offset += line_len;
                    continue;
                }
            };

            // Skip keywords inside strings/heredocs/regexps/symbols
            if !code_map.is_not_string(byte_offset + trimmed_start) {
                byte_offset += line_len;
                continue;
            }

            let kw_str = std::str::from_utf8(keyword).unwrap_or("rescue");

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

            byte_offset += line_len;
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        EmptyLinesAroundExceptionHandlingKeywords,
        "cops/layout/empty_lines_around_exception_handling_keywords"
    );

    #[test]
    fn skip_keywords_in_heredoc() {
        let source = b"x = <<~RUBY\n  begin\n    something\n\n  rescue\n\n    handle\n  end\nRUBY\n";
        let diags = run_cop_full(&EmptyLinesAroundExceptionHandlingKeywords, source);
        assert!(diags.is_empty(), "Should not fire on rescue inside heredoc, got: {:?}", diags);
    }

    #[test]
    fn skip_keywords_in_string() {
        let source = b"x = \"rescue\"\ny = 'ensure'\n";
        let diags = run_cop_full(&EmptyLinesAroundExceptionHandlingKeywords, source);
        assert!(diags.is_empty(), "Should not fire on keywords inside strings");
    }
}
