use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DependencyVersion;

const DEP_METHODS: &[&str] = &[
    ".add_dependency",
    ".add_runtime_dependency",
    ".add_development_dependency",
];

impl Cop for DependencyVersion {
    fn name(&self) -> &'static str {
        "Gemspec/DependencyVersion"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec"]
    }

    fn check_lines(&self, source: &SourceFile, config: &CopConfig) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "required");
        let allowed_gems = config
            .get_string_array("AllowedGems")
            .unwrap_or_default();

        let mut diagnostics = Vec::new();

        for (line_idx, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            if trimmed.starts_with('#') {
                continue;
            }

            for &method in DEP_METHODS {
                if let Some(pos) = line_str.find(method) {
                    let after = &line_str[pos + method.len()..];
                    let (gem_name, has_version) = parse_dependency_args(after);

                    // Check if gem is in allowed list
                    if let Some(ref name) = gem_name {
                        if allowed_gems.iter().any(|g| g == name) {
                            continue;
                        }
                    }

                    match style {
                        "required" => {
                            if !has_version {
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line_idx + 1,
                                    pos + 1, // skip the dot
                                    "Dependency version is required.".to_string(),
                                ));
                            }
                        }
                        "forbidden" => {
                            if has_version {
                                diagnostics.push(self.diagnostic(
                                    source,
                                    line_idx + 1,
                                    pos + 1, // skip the dot
                                    "Dependency version should not be specified.".to_string(),
                                ));
                            }
                        }
                        _ => {}
                    }
                    break; // Only match one method per line
                }
            }
        }
        diagnostics
    }
}

/// Parse dependency method arguments to extract gem name and whether a version is present.
/// Handles patterns like:
///   ('gem_name', '~> 1.0')
///   'gem_name', '>= 2.0'
///   ('gem_name')
///   'gem_name'
fn parse_dependency_args(after_method: &str) -> (Option<String>, bool) {
    let s = after_method.trim_start();
    let s = if s.starts_with('(') {
        s[1..].trim_start()
    } else {
        s
    };

    // Extract gem name
    let gem_name = if s.starts_with('\'') || s.starts_with('"') {
        let quote = s.as_bytes()[0];
        let rest = &s[1..];
        rest.find(|c: char| c as u8 == quote).map(|end| {
            let name = rest[..end].to_string();
            (name, &rest[end + 1..])
        })
    } else {
        None
    };

    let (name, remainder) = match gem_name {
        Some((n, r)) => (Some(n), r),
        None => (None, s),
    };

    // Check if there's a version argument after the gem name
    let remainder = remainder.trim_start();
    let has_version = if remainder.starts_with(',') {
        let after_comma = remainder[1..].trim_start();
        // Check for a version string: starts with quote containing version-like content
        is_version_string(after_comma)
    } else {
        false
    };

    (name, has_version)
}

/// Check if the string starts with a quoted version specifier.
fn is_version_string(s: &str) -> bool {
    if s.starts_with('\'') || s.starts_with('"') {
        let quote = s.as_bytes()[0];
        let rest = &s[1..];
        if let Some(end) = rest.find(|c: char| c as u8 == quote) {
            let content = &rest[..end];
            // Version strings typically start with optional operator and digits
            let trimmed = content.trim();
            return !trimmed.is_empty()
                && (trimmed.as_bytes()[0].is_ascii_digit()
                    || trimmed.starts_with(">=")
                    || trimmed.starts_with("~>")
                    || trimmed.starts_with("<=")
                    || trimmed.starts_with("!=")
                    || trimmed.starts_with('>')
                    || trimmed.starts_with('<')
                    || trimmed.starts_with('='));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DependencyVersion, "cops/gemspec/dependency_version");
}
