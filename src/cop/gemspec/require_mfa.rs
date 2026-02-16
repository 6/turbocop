use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RequireMfa;

impl Cop for RequireMfa {
    fn name(&self) -> &'static str {
        "Gemspec/RequireMFA"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut found_mfa = false;

        for line in source.lines() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            if trimmed.starts_with('#') {
                continue;
            }

            // Check for metadata['rubygems_mfa_required'] or metadata["rubygems_mfa_required"]
            if (trimmed.contains("metadata['rubygems_mfa_required']")
                || trimmed.contains("metadata[\"rubygems_mfa_required\"]"))
                && trimmed.contains("= ")
            {
                // Check if set to 'true'
                if trimmed.contains("'true'") || trimmed.contains("\"true\"") {
                    found_mfa = true;
                }
            }
        }

        if !found_mfa {
            // Report at line 1, column 0
            vec![self.diagnostic(
                source,
                1,
                0,
                "`rubygems_mfa_required` must be set to `'true'` in gemspec metadata.".to_string(),
            )]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        RequireMfa, "cops/gemspec/require_mfa",
        missing_metadata = "missing_metadata.rb",
        wrong_value = "wrong_value.rb",
        no_metadata_at_all = "no_metadata_at_all.rb",
    );
}
