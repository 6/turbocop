use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Gemspec/DependencyVersion
///
/// Investigated: FP from Gem::Specification.new with positional args (RuboCop skips
/// these blocks entirely via GemspecHelp NodePattern). FN from interpolated strings
/// like `"~> #{VERSION}"` being treated as version specifiers (RuboCop only considers
/// plain `str` nodes, not `dstr`/interpolated strings).
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

    fn default_enabled(&self) -> bool {
        false
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
        let style = config.get_str("EnforcedStyle", "required");
        let allowed_gems = config.get_string_array("AllowedGems").unwrap_or_default();

        // RuboCop only checks dependencies inside Gem::Specification.new blocks
        // WITHOUT positional arguments. If .new has positional args (e.g.,
        // `Gem::Specification.new 'name', '1.0' do |s|`), the entire file is skipped.
        if has_positional_args_spec_new(source) {
            return;
        }

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
    }
}

/// Check if the file contains `Gem::Specification.new` with positional arguments.
/// RuboCop's GemspecHelp only matches `.new` followed immediately by a block (no args).
/// Forms like `Gem::Specification.new 'name', '1.0' do |s|` have positional args
/// and RuboCop skips the entire block. Also handles variable args like
/// `Gem::Specification.new name, Version do |s|`.
fn has_positional_args_spec_new(source: &SourceFile) -> bool {
    for line in source.lines() {
        let line_str = match std::str::from_utf8(line) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Some(pos) = line_str.find("Gem::Specification.new") {
            let after = line_str[pos + "Gem::Specification.new".len()..].trim_start();
            // RuboCop requires .new followed directly by a block (do/{ with no args).
            // If .new is followed by anything other than `do`, `{`, `(`, or end-of-line,
            // it has positional arguments.
            // Note: `(` could be `Gem::Specification.new(&block)` but we handle common
            // patterns: if after `(` there's no `&`, it's likely positional args.
            if after.is_empty() || after.starts_with("do") || after.starts_with('{') {
                continue;
            }
            if after.starts_with('(') {
                // `Gem::Specification.new(&block)` - no positional args
                // `Gem::Specification.new('name', ...)` - positional args
                let inner = after[1..].trim_start();
                if inner.starts_with('&') {
                    continue;
                }
                return true;
            }
            // Anything else (string literal, variable, constant) = positional args
            return true;
        }
    }
    false
}

/// Parse dependency method arguments to extract gem name and whether a version is present.
/// Handles patterns like:
///   ('gem_name', '~> 1.0')
///   'gem_name', '>= 2.0'
///   ('gem_name')
///   'gem_name'
fn parse_dependency_args(after_method: &str) -> (Option<String>, bool) {
    let s = after_method.trim_start();
    let s = if let Some(stripped) = s.strip_prefix('(') {
        stripped.trim_start()
    } else {
        s
    };

    // Extract gem name from quoted string or percent string literal
    let gem_name = if s.starts_with('\'') || s.starts_with('"') {
        let quote = s.as_bytes()[0];
        let rest = &s[1..];
        rest.find(|c: char| c as u8 == quote).map(|end| {
            let name = rest[..end].to_string();
            (name, &rest[end + 1..])
        })
    } else {
        try_parse_percent_string(s)
    };

    let (name, remainder) = match gem_name {
        Some((n, r)) => (Some(n), r),
        None => (None, s),
    };

    // Check if there's a version argument after the gem name
    let remainder = remainder.trim_start();
    let has_version = if let Some(stripped) = remainder.strip_prefix(',') {
        let after_comma = stripped.trim_start();
        // Check for a version string: starts with quote containing version-like content
        is_version_string(after_comma)
    } else {
        false
    };

    (name, has_version)
}

/// Try to parse a Ruby percent string literal (%q<...>, %q(...), %q[...], %Q<...>, %Q(...), %Q[...]).
/// Returns (extracted_string, remainder_after_closing_delimiter) if successful.
fn try_parse_percent_string(s: &str) -> Option<(String, &str)> {
    let bytes = s.as_bytes();
    if bytes.len() < 4 || bytes[0] != b'%' {
        return None;
    }
    // Accept %q or %Q
    if bytes[1] != b'q' && bytes[1] != b'Q' {
        return None;
    }
    let open = bytes[2];
    let close = match open {
        b'<' => b'>',
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        _ => return None,
    };
    let rest = &s[3..];
    rest.find(|c: char| c as u8 == close).map(|end| {
        let name = rest[..end].to_string();
        (name, &rest[end + 1..])
    })
}

/// Check if the string starts with a quoted version specifier.
/// RuboCop only considers plain string nodes (`str`), not interpolated strings (`dstr`).
/// So `"~> #{VERSION}"` does NOT count as a version specifier.
fn is_version_string(s: &str) -> bool {
    if s.starts_with('\'') || s.starts_with('"') {
        let quote = s.as_bytes()[0];
        let rest = &s[1..];
        if let Some(end) = rest.find(|c: char| c as u8 == quote) {
            let content = &rest[..end];
            // Interpolated strings (containing #{...}) are not plain string nodes
            // and RuboCop does not treat them as version specifiers
            if content.contains("#{") {
                return false;
            }
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

    #[test]
    fn positional_args_string_literal_skipped() {
        // Gem::Specification.new with string literal positional args — RuboCop skips
        let source = crate::parse::source::SourceFile::from_bytes(
            "example.gemspec",
            b"Gem::Specification.new 'example', '1.0' do |s|\n  s.add_dependency 'foo'\nend\n"
                .to_vec(),
        );
        let config = crate::cop::CopConfig::default();
        let mut diags = vec![];
        DependencyVersion.check_lines(&source, &config, &mut diags, None);
        assert!(
            diags.is_empty(),
            "should skip file with positional args: {diags:?}"
        );
    }

    #[test]
    fn positional_args_variable_skipped() {
        // Gem::Specification.new with variable positional args — also skipped
        let source = crate::parse::source::SourceFile::from_bytes(
            "example.gemspec",
            b"Gem::Specification.new name, VERSION do |s|\n  s.add_dependency 'foo'\nend\n"
                .to_vec(),
        );
        let config = crate::cop::CopConfig::default();
        let mut diags = vec![];
        DependencyVersion.check_lines(&source, &config, &mut diags, None);
        assert!(
            diags.is_empty(),
            "should skip file with variable positional args: {diags:?}"
        );
    }

    #[test]
    fn interpolated_version_not_counted() {
        // Interpolated version strings should NOT count as version specifiers
        assert!(!is_version_string("\"~> #{VERSION}\""));
        assert!(!is_version_string("\"~> #{Foo::VERSION}\""));
        // Plain version strings should still count
        assert!(is_version_string("'~> 1.0'"));
        assert!(is_version_string("'>= 2.0'"));
    }
}
