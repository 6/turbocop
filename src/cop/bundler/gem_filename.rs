use std::path::Path;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct GemFilename;

impl Cop for GemFilename {
    fn name(&self) -> &'static str {
        "Bundler/GemFilename"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/Gemfile", "**/gems.rb"]
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "Gemfile");
        let path = Path::new(source.path_str());
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match enforced_style {
            "Gemfile" => {
                if file_name == "gems.rb" {
                    return vec![self.diagnostic(
                        source,
                        1,
                        0,
                        format!(
                            "`gems.rb` file was found but `Gemfile` is required (file path: {}).",
                            source.path_str()
                        ),
                    )];
                }
                if file_name == "gems.locked" {
                    return vec![self.diagnostic(
                        source,
                        1,
                        0,
                        format!(
                            "Expected a `Gemfile.lock` with `Gemfile` but found `gems.locked` file (file path: {}).",
                            source.path_str()
                        ),
                    )];
                }
            }
            "gems.rb" => {
                if file_name == "Gemfile" {
                    return vec![self.diagnostic(
                        source,
                        1,
                        0,
                        format!(
                            "`Gemfile` was found but `gems.rb` is required (file path: {}).",
                            source.path_str()
                        ),
                    )];
                }
                if file_name == "Gemfile.lock" {
                    return vec![self.diagnostic(
                        source,
                        1,
                        0,
                        format!(
                            "Expected a `gems.locked` with `gems.rb` but found `Gemfile.lock` file (file path: {}).",
                            source.path_str()
                        ),
                    )];
                }
            }
            _ => {}
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_scenario_fixture_tests!(
        GemFilename, "cops/bundler/gem_filename",
        gems_rb_when_gemfile_enforced = "gems_rb_when_gemfile_enforced.rb",
        gems_locked_when_gemfile_enforced = "gems_locked_when_gemfile_enforced.rb",
        nested_gems_rb = "nested_gems_rb.rb",
    );
}
