use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TripleQuotes;

impl Cop for TripleQuotes {
    fn name(&self) -> &'static str {
        "Lint/TripleQuotes"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed_start = line
                .iter()
                .position(|&b| b != b' ' && b != b'\t')
                .unwrap_or(line.len());
            let trimmed = &line[trimmed_start..];

            // Check for lines that contain triple quotes (""" or ''')
            // We need to find these patterns anywhere on the line
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
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(TripleQuotes, "cops/lint/triple_quotes");
}
