use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle reported FP=4, FN=6.
///
/// FP=4: Fixed by:
///   1. Requiring `Gem::Specification.new` block in file before checking deps (matches
///      RuboCop's `match_block_variable_name?` which requires the receiver to be the
///      block variable). Files without `Gem::Specification.new` are now skipped.
///   2. Scanning ALL string literals in args for version specs, not just the second arg
///      (matches RuboCop's `<(str #version_specification?) ...>` which checks all args).
///      This handles ENV.fetch with a third version arg, and variable first args in
///      `.each` blocks where the version is a later argument.
///
/// FN=6: Fixed by:
///   3 FN from `!=` operator: RuboCop's VERSION_SPECIFICATION_REGEX `/^\s*[~<>=]*\s*[0-9.]+/`
///   does NOT include `!` in its character class. So `'!= 0.3.1'` is not a version spec.
///   Removed `!=` from nitrocop's version operator list.
///   3 FN from pagy: Likely corpus state discrepancy (local run detects them correctly).
///
/// Prior fix: FP from Gem::Specification.new with positional args (RuboCop skips
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
        // If there's no Gem::Specification.new at all, the file is also skipped.
        if !should_check_dependencies(source) {
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

/// Check whether dependencies should be checked in this file.
///
/// Returns true only if the file contains `Gem::Specification.new` followed by a block
/// (do or {) with no positional arguments. This matches RuboCop's GemspecHelp
/// `gem_specification` NodePattern which requires `.new` with only a block parameter.
///
/// Returns false (skip file) when:
/// - No `Gem::Specification.new` found at all
/// - `Gem::Specification.new` has positional arguments
fn should_check_dependencies(source: &SourceFile) -> bool {
    for line in source.lines() {
        let line_str = match std::str::from_utf8(line) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Some(pos) = line_str.find("Gem::Specification.new") {
            let after = line_str[pos + "Gem::Specification.new".len()..].trim_start();
            // RuboCop requires .new followed directly by a block (do/{ with no args).
            if after.is_empty() || after.starts_with("do") || after.starts_with('{') {
                return true;
            }
            if let Some(stripped) = after.strip_prefix('(') {
                // `Gem::Specification.new(&block)` - no positional args → check deps
                // `Gem::Specification.new('name', ...)` - positional args → skip
                let inner = stripped.trim_start();
                if inner.starts_with('&') {
                    return true;
                }
                return false;
            }
            // Anything else (string literal, variable, constant) = positional args → skip
            return false;
        }
    }
    // No Gem::Specification.new found → no block variable → RuboCop wouldn't check deps
    false
}

/// Parse dependency method arguments to extract gem name and whether a version is present.
///
/// This follows RuboCop's semantics:
/// - Gem name is extracted from the first string/percent-string literal (if present)
/// - Version is detected if ANY string literal in the args matches RuboCop's
///   VERSION_SPECIFICATION_REGEX: `/^\s*[~<>=]*\s*[0-9.]+/`
///   This handles: multiple args, variables mixed with strings, ENV.fetch patterns, etc.
fn parse_dependency_args(after_method: &str) -> (Option<String>, bool) {
    let s = after_method.trim_start();
    let s = if let Some(stripped) = s.strip_prefix('(') {
        stripped.trim_start()
    } else {
        s
    };

    // Extract gem name from first string literal or percent string
    let gem_name = extract_first_string(s);

    // Check if ANY string literal in the args matches the version spec regex.
    // This matches RuboCop's `<(str #version_specification?) ...>` pattern
    // which checks all arguments, not just the second one.
    let has_version = has_any_version_string(s);

    (gem_name, has_version)
}

/// Extract the first string literal from the arguments (gem name).
fn extract_first_string(s: &str) -> Option<String> {
    if s.starts_with('\'') || s.starts_with('"') {
        let quote = s.as_bytes()[0];
        let rest = &s[1..];
        rest.find(|c: char| c as u8 == quote)
            .map(|end| rest[..end].to_string())
    } else {
        try_parse_percent_string(s).map(|(name, _)| name)
    }
}

/// Check if any string literal in the text matches the version specification regex.
///
/// Scans all single- and double-quoted strings. Skips the first string (gem name)
/// since gem names like 'rails' don't match the version regex anyway (no leading digit
/// or operator), but version strings like '>= 1.0' do.
///
/// Matches RuboCop's `/^\s*[~<>=]*\s*[0-9.]+/` applied to each `(str ...)` node.
fn has_any_version_string(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() {
        if bytes[pos] == b'\'' || bytes[pos] == b'"' {
            let quote = bytes[pos];
            let start = pos + 1;
            // Find closing quote
            let mut end = start;
            while end < bytes.len() && bytes[end] != quote {
                end += 1;
            }
            if end < bytes.len() {
                let content = &s[start..end];
                if is_version_content(content) {
                    return true;
                }
                pos = end + 1;
            } else {
                break; // Unclosed quote
            }
        } else {
            pos += 1;
        }
    }
    false
}

/// Check if a string's content matches RuboCop's VERSION_SPECIFICATION_REGEX.
///
/// Pattern: `/^\s*[~<>=]*\s*[0-9.]+/`
///
/// Note: `!` is NOT in the character class `[~<>=]`, so `!= 1.0` does NOT match.
/// Interpolated strings (containing `#{...}`) are also excluded — RuboCop only
/// matches plain `str` nodes, not `dstr`/interpolated strings.
fn is_version_content(content: &str) -> bool {
    if content.contains("#{") {
        return false;
    }
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Skip optional version operators: only ~, <, >, = (NOT !)
    let after_ops = trimmed.trim_start_matches(['~', '<', '>', '=']);
    let after_space = after_ops.trim_start();
    // Must start with a digit
    after_space
        .as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_digit())
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
    fn no_spec_new_block_skipped() {
        // File without Gem::Specification.new — RuboCop wouldn't check deps
        let source = crate::parse::source::SourceFile::from_bytes(
            "example.gemspec",
            b"spec.add_dependency 'foo'\n".to_vec(),
        );
        let config = crate::cop::CopConfig::default();
        let mut diags = vec![];
        DependencyVersion.check_lines(&source, &config, &mut diags, None);
        assert!(
            diags.is_empty(),
            "should skip file without Gem::Specification.new: {diags:?}"
        );
    }

    #[test]
    fn interpolated_version_not_counted() {
        // Interpolated version strings should NOT count as version specifiers
        assert!(!is_version_content("~> #{VERSION}"));
        assert!(!is_version_content("~> #{Foo::VERSION}"));
        // Plain version strings should still count
        assert!(is_version_content("~> 1.0"));
        assert!(is_version_content(">= 2.0"));
    }

    #[test]
    fn not_equal_not_a_version_spec() {
        // != is NOT a version operator per RuboCop's regex
        assert!(!is_version_content("!= 0.3.1"));
        assert!(!is_version_content("!= 1.8.8"));
        // But these ARE valid version specs
        assert!(is_version_content(">= 1.0"));
        assert!(is_version_content("~> 2.0"));
        assert!(is_version_content("< 3.0"));
        assert!(is_version_content("= 1.0"));
        assert!(is_version_content("1.0"));
    }

    #[test]
    fn not_equal_flagged_as_no_version() {
        // `!= 1.8.8` is NOT a version spec, so the dep should be flagged
        let source = crate::parse::source::SourceFile::from_bytes(
            "example.gemspec",
            b"Gem::Specification.new do |s|\n  s.add_dependency('i18n', '!= 1.8.8')\nend\n"
                .to_vec(),
        );
        let config = crate::cop::CopConfig::default();
        let mut diags = vec![];
        DependencyVersion.check_lines(&source, &config, &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "should flag dep with != as no version: {diags:?}"
        );
    }

    #[test]
    fn has_any_version_finds_later_args() {
        // Version string as third arg should be detected
        assert!(has_any_version_string(
            "'client', ENV.fetch('VER', '>= 1.0'), '< 3.0'"
        ));
        // Variable first arg with version second arg
        assert!(has_any_version_string("comp, '>= 6.1.4'"));
        // No version at all
        assert!(!has_any_version_string("'foo'"));
        // Only gem name
        assert!(!has_any_version_string("'rails'"));
    }
}
