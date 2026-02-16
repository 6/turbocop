use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RequiredRubyVersion;

impl Cop for RequiredRubyVersion {
    fn name(&self) -> &'static str {
        "Gemspec/RequiredRubyVersion"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut found = false;

        for line in source.lines() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            if trimmed.starts_with('#') {
                continue;
            }

            // Check for required_ruby_version assignment
            if trimmed.contains(".required_ruby_version") {
                let after = trimmed
                    .split(".required_ruby_version")
                    .nth(1)
                    .unwrap_or("");
                let after_trimmed = after.trim_start();
                // Must be an assignment (= or >=) not just a method call check
                if after_trimmed.starts_with('=') || after_trimmed.is_empty() {
                    found = true;
                    break;
                }
            }
        }

        if !found {
            vec![self.diagnostic(
                source,
                1,
                0,
                "`required_ruby_version` should be set in gemspec.".to_string(),
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
        RequiredRubyVersion, "cops/gemspec/required_ruby_version",
        missing_version = "missing_version.rb",
        empty_gemspec = "empty_gemspec.rb",
        only_other_attrs = "only_other_attrs.rb",
    );
}
