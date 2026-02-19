use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

use super::extract_gem_name;

pub struct DuplicatedGem;

impl Cop for DuplicatedGem {
    fn name(&self) -> &'static str {
        "Bundler/DuplicatedGem"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig, diagnostics: &mut Vec<Diagnostic>) {
        // Maps gem name -> first occurrence line (1-indexed)
        let mut seen: HashMap<String, usize> = HashMap::new();
        // Track conditional nesting depth for if/elsif/else/case/when
        let mut in_conditional = false;
        let mut conditional_depth = 0;

        for (i, line) in source.lines().enumerate() {
            let line_str = std::str::from_utf8(line).unwrap_or("");
            let trimmed = line_str.trim();

            // Track conditional blocks (simple heuristic)
            if trimmed.starts_with("if ")
                || trimmed.starts_with("unless ")
                || trimmed.starts_with("case")
            {
                if !in_conditional {
                    in_conditional = true;
                    conditional_depth += 1;
                }
            } else if trimmed == "end" && in_conditional {
                conditional_depth -= 1;
                if conditional_depth == 0 {
                    in_conditional = false;
                }
            }

            if let Some(gem_name) = extract_gem_name(line_str) {
                let line_num = i + 1;
                if let Some(&first_line) = seen.get(gem_name) {
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        0,
                        format!(
                            "Gem `{}` requirements already given on line {} of the Gemfile.",
                            gem_name, first_line
                        ),
                    ));
                } else {
                    seen.insert(gem_name.to_string(), line_num);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicatedGem, "cops/bundler/duplicated_gem");
}
