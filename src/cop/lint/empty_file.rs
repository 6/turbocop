use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyFile;

impl Cop for EmptyFile {
    fn name(&self) -> &'static str {
        "Lint/EmptyFile"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig, diagnostics: &mut Vec<Diagnostic>) {
        let allow_comments = config.get_bool("AllowComments", true);

        let src = source.as_bytes();
        if src.is_empty() {
            diagnostics.push(self.diagnostic(source, 1, 0, "Empty file detected.".to_string()));
            return;
        }

        // Check if file has only whitespace and optionally comments
        let mut has_code = false;
        let mut has_comments = false;

        for line in source.lines() {
            let trimmed = line
                .iter()
                .position(|&b| b != b' ' && b != b'\t' && b != b'\r')
                .map(|start| &line[start..])
                .unwrap_or(&[]);

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with(b"#") {
                has_comments = true;
                continue;
            }

            has_code = true;
            break;
        }

        if has_code {
            return;
        }

        if has_comments && allow_comments {
            return;
        }

        diagnostics.push(self.diagnostic(source, 1, 0, "Empty file detected.".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        EmptyFile, "cops/lint/empty_file",
        empty_file = "empty.rb",
        whitespace_only = "whitespace_only.rb",
        blank_lines = "blank_lines.rb",
    );
}
