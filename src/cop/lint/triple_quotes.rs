use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct TripleQuotes;

impl Cop for TripleQuotes {
    fn name(&self) -> &'static str {
        "Lint/TripleQuotes"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut byte_offset: usize = 0;

        for (i, line) in source.lines().enumerate() {
            let line_len = line.len() + 1; // +1 for newline
            let trimmed_start = line
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .unwrap_or(line.len());
            let trimmed = &line[trimmed_start..];

            // Skip comment lines — triple quotes in documentation examples are not offenses
            if trimmed.starts_with(b"#") {
                byte_offset += line_len;
                continue;
            }

            // Skip lines inside string/heredoc regions
            if !code_map.is_not_string(byte_offset + trimmed_start) {
                byte_offset += line_len;
                continue;
            }

            // Check for lines that contain triple quotes (""" or ''')
            for j in 0..trimmed.len().saturating_sub(2) {
                if (trimmed[j] == b'"' && trimmed[j + 1] == b'"' && trimmed[j + 2] == b'"')
                    || (trimmed[j] == b'\'' && trimmed[j + 1] == b'\'' && trimmed[j + 2] == b'\'')
                {
                    diagnostics.push(self.diagnostic(
                        source,
                        i + 1,
                        trimmed_start + j,
                        "Triple quotes found. Did you mean to use a heredoc?".to_string(),
                    ));
                    break; // Only one per line
                }
            }

            byte_offset += line_len;
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TripleQuotes, "cops/lint/triple_quotes");

    #[test]
    fn skip_in_heredoc() {
        let source = b"x = <<~RUBY\n  \"\"\"\n  foo\n  \"\"\"\nRUBY\n";
        let diags = crate::testutil::run_cop_full(&TripleQuotes, source);
        assert!(diags.is_empty(), "Should not fire on triple quotes inside heredoc");
    }
}
