use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DevelopmentDependencies;

impl Cop for DevelopmentDependencies {
    fn name(&self) -> &'static str {
        "Gemspec/DevelopmentDependencies"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "Gemfile");
        let allowed_gems = config.get_string_array("AllowedGems").unwrap_or_default();

        // When style is "gemspec", development dependencies belong in gemspec, so no offense
        if style == "gemspec" {
            return;
        }

        // For "Gemfile" or "gems.rb" styles, flag add_development_dependency calls
        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            if let Some(pos) = line_str.find(".add_development_dependency") {
                // Check if the gem is in the allowed list
                let after_method = &line_str[pos + ".add_development_dependency".len()..];
                if is_gem_allowed(after_method, &allowed_gems) {
                    continue;
                }
                diagnostics.push(self.diagnostic(
                    source,
                    line_idx + 1,
                    pos + 1, // skip the dot
                    format!("Specify development dependencies in `{style}` instead of gemspec."),
                ));
            }
        }
    }
}

/// Check if the gem name following the method call is in the allowed list.
fn is_gem_allowed(after_method: &str, allowed_gems: &[String]) -> bool {
    if allowed_gems.is_empty() {
        return false;
    }
    // Try to extract gem name from patterns like:
    //   ('gem_name', ...) or  'gem_name' or "gem_name"
    let trimmed = after_method.trim_start();
    let trimmed = if trimmed.starts_with('(') {
        trimmed[1..].trim_start()
    } else {
        trimmed
    };
    let gem_name = if trimmed.starts_with('\'') || trimmed.starts_with('"') {
        let quote = trimmed.as_bytes()[0];
        let rest = &trimmed[1..];
        rest.find(|c: char| c as u8 == quote)
            .map(|end| &rest[..end])
    } else {
        None
    };
    if let Some(name) = gem_name {
        allowed_gems.iter().any(|g| g == name)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DevelopmentDependencies,
        "cops/gemspec/development_dependencies"
    );
}
