use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::extract_gem_name;

pub struct GemVersion;

impl Cop for GemVersion {
    fn name(&self) -> &'static str {
        "Bundler/GemVersion"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig, diagnostics: &mut Vec<Diagnostic>, _corrections: Option<&mut Vec<crate::correction::Correction>>) {
        let enforced_style = config.get_str("EnforcedStyle", "required");
        let allowed_gems = config
            .get_string_array("AllowedGems")
            .unwrap_or_default();


        for (i, line) in source.lines().enumerate() {
            let line_str = std::str::from_utf8(line).unwrap_or("");

            if let Some(gem_name) = extract_gem_name(line_str) {
                // Skip allowed gems
                if allowed_gems.iter().any(|g| g == gem_name) {
                    continue;
                }

                let has_version = has_version_or_source_specifier(line_str);
                let line_num = i + 1;

                match enforced_style {
                    "required" => {
                        if !has_version {
                            diagnostics.push(self.diagnostic(
                                source,
                                line_num,
                                0,
                                "Gem version specification is required.".to_string(),
                            ));
                        }
                    }
                    "forbidden" => {
                        if has_version {
                            diagnostics.push(self.diagnostic(
                                source,
                                line_num,
                                0,
                                "Gem version specification is forbidden.".to_string(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Check if a gem declaration has a version specifier or source alternative
/// (git:, github:, branch:, ref:, tag:).
fn has_version_or_source_specifier(line: &str) -> bool {
    let trimmed = line.trim();

    // Find the end of the gem name (closing quote)
    let first_quote = match trimmed.find(|c: char| c == '\'' || c == '"') {
        Some(idx) => idx,
        None => return false,
    };
    let quote_char = trimmed.as_bytes()[first_quote];
    let after_name_start = first_quote + 1;
    let name_end = match trimmed[after_name_start..].find(|c: char| c as u8 == quote_char) {
        Some(idx) => after_name_start + idx + 1,
        None => return false,
    };

    let rest = trimmed[name_end..].trim();

    // No arguments after gem name
    if rest.is_empty() {
        return false;
    }

    // Must have a comma to have additional arguments
    let rest = if rest.starts_with(',') {
        rest[1..].trim()
    } else {
        return false;
    };

    // Check for source alternatives: git:, github:, branch:, ref:, tag:
    let source_keywords = ["git:", "github:", "branch:", "ref:", "tag:"];
    for keyword in &source_keywords {
        if rest.contains(keyword) {
            return true;
        }
    }

    // Check for version string — first non-keyword argument that is a quoted version
    // A version string starts with optional operator then digits
    if rest.starts_with('\'') || rest.starts_with('"') {
        let q = rest.as_bytes()[0];
        if let Some(end) = rest[1..].find(|c: char| c as u8 == q) {
            let val = &rest[1..1 + end];
            if is_version_string(val) {
                return true;
            }
        }
    }

    // Check if there's a keyword-only argument like `require: false` (not a version)
    // If we get here and haven't found a version, there's no version
    false
}

/// Check if a string looks like a version specifier.
fn is_version_string(s: &str) -> bool {
    let s = s.trim();
    let s = s
        .trim_start_matches("~>")
        .trim_start_matches(">=")
        .trim_start_matches("<=")
        .trim_start_matches("!=")
        .trim_start_matches('>')
        .trim_start_matches('<')
        .trim_start_matches('=')
        .trim();
    s.starts_with(|c: char| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(GemVersion, "cops/bundler/gem_version");
}
