use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::extract_gem_name;

pub struct GemComment;

// Known corpus gap (1 FN as of 2026-03-02):
// Landed fix: ignore magic comments as gem description comments.
// Attempted fix (reverted): ignore preceding comments on modifier-guarded gem declarations.
// Effect: FP regressions (FP=0/FN=1 → FP=13/FN=0). Needs AST comment-association
// parity with RuboCop instead of line-adjacent heuristics.

impl Cop for GemComment {
    fn name(&self) -> &'static str {
        "Bundler/GemComment"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let ignored_gems = config.get_string_array("IgnoredGems").unwrap_or_default();
        let only_for = config.get_string_array("OnlyFor").unwrap_or_default();
        let check_version_specifiers = only_for.iter().any(|s| s == "version_specifiers");

        let lines: Vec<&[u8]> = source.lines().collect();
        let mut in_block_comment = false;

        for (i, line) in lines.iter().enumerate() {
            let line_str = std::str::from_utf8(line).unwrap_or("");
            let trimmed = line_str.trim_start();

            if in_block_comment {
                if trimmed.starts_with("=end") {
                    in_block_comment = false;
                }
                continue;
            }
            if trimmed.starts_with("=begin") {
                if !trimmed.contains("=end") {
                    in_block_comment = true;
                }
                continue;
            }

            if let Some(gem_name) = extract_gem_name(line_str) {
                // Skip ignored gems
                if ignored_gems.iter().any(|g| g == gem_name) {
                    continue;
                }

                // When OnlyFor includes "version_specifiers", only flag gems with version constraints
                if check_version_specifiers && !has_version_specifier(line_str) {
                    continue;
                }

                // Check if the preceding line is a comment, or this line has an inline comment
                let has_comment = has_inline_comment(line_str)
                    || (i > 0
                        && std::str::from_utf8(lines[i - 1])
                            .unwrap_or("")
                            .trim()
                            .starts_with('#')
                        && !is_magic_comment(
                            std::str::from_utf8(lines[i - 1]).unwrap_or("").trim(),
                        ));

                if !has_comment {
                    let line_num = i + 1;
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        0,
                        "Missing gem description comment.".to_string(),
                    ));
                }
            }
        }
    }
}

/// Check if a gem declaration line has a version specifier.
/// Version specifiers look like: '~> 1.0', '>= 2.0', '1.0', etc.
fn has_version_specifier(line: &str) -> bool {
    let trimmed = line.trim();
    // After `gem 'name'`, look for version-like arguments
    // Find the closing quote of the gem name
    let first_quote = match trimmed.find(['\'', '"']) {
        Some(idx) => idx,
        None => return false,
    };
    let quote_char = trimmed.as_bytes()[first_quote];
    let after_name_start = first_quote + 1;
    let name_end = match trimmed[after_name_start..].find(|c: char| c as u8 == quote_char) {
        Some(idx) => after_name_start + idx + 1,
        None => return false,
    };

    let rest = &trimmed[name_end..];
    // Look for version string patterns after a comma
    // A version string starts with optional operator (>=, ~>, <=, >, <, =, !=) then digits
    if let Some(comma_idx) = rest.find(',') {
        let after_comma = rest[comma_idx + 1..].trim();
        // Check if next argument is a quoted version string
        if after_comma.starts_with('\'') || after_comma.starts_with('"') {
            let q = after_comma.as_bytes()[0];
            if let Some(end) = after_comma[1..].find(|c: char| c as u8 == q) {
                let val = &after_comma[1..1 + end];
                if is_version_string(val) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a string looks like a version specifier.
/// Examples: "1.0", "~> 1.0", ">= 2.0", "< 3.0"
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
    // Should start with a digit
    s.starts_with(|c: char| c.is_ascii_digit())
}

/// Check if the line has an inline comment (# after the gem declaration).
fn has_inline_comment(line: &str) -> bool {
    // Simple heuristic: look for # that's not inside quotes
    let mut in_single = false;
    let mut in_double = false;
    for ch in line.chars() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double => return true,
            _ => {}
        }
    }
    false
}

fn is_magic_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return false;
    }

    let body = trimmed.trim_start_matches('#').trim_start();
    body.starts_with("frozen_string_literal:")
        || body.starts_with("encoding:")
        || body.starts_with("coded by:")
        || body.starts_with("-*-")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(GemComment, "cops/bundler/gem_comment");
}
