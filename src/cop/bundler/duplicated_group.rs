use std::collections::HashMap;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DuplicatedGroup;

/// Extract sorted group names from a `group` declaration line.
/// Handles: `group :dev do`, `group :dev, :test do`, `group 'dev' do`
/// Returns None if this is not a group declaration line.
fn extract_group_key(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("group ") && !trimmed.starts_with("group(") {
        return None;
    }
    // Must end with `do` to be a block-style group
    if !trimmed.ends_with(" do") && !trimmed.ends_with(" do") {
        // Also check without trailing comment
        let code_part = if let Some(idx) = trimmed.find('#') {
            trimmed[..idx].trim()
        } else {
            trimmed
        };
        if !code_part.ends_with(" do") {
            return None;
        }
    }

    // Extract the part between `group` and `do`
    let start = if trimmed.starts_with("group(") {
        6
    } else {
        6 // "group "
    };
    let end = trimmed.rfind(" do")?;
    let args_str = &trimmed[start..end];

    // Parse group names — symbols (:name) and strings ('name' or "name")
    let mut groups: Vec<String> = Vec::new();
    let mut rest = args_str.trim();

    while !rest.is_empty() {
        // Skip leading comma and whitespace
        rest = rest.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
        if rest.is_empty() {
            break;
        }

        // If we hit a keyword arg (foo: ...) that is not a group name, stop
        if rest.contains(':') && !rest.starts_with(':') {
            // This is something like `foo: true` — stop parsing groups
            break;
        }

        if rest.starts_with(':') {
            // Symbol argument :name
            let name_end = rest[1..]
                .find(|c: char| c == ',' || c.is_whitespace())
                .map(|i| i + 1)
                .unwrap_or(rest.len());
            let name = &rest[1..name_end];
            groups.push(name.to_string());
            rest = &rest[name_end..];
        } else if rest.starts_with('\'') || rest.starts_with('"') {
            let quote = rest.as_bytes()[0];
            if let Some(end_idx) = rest[1..].find(|c: char| c as u8 == quote) {
                let name = &rest[1..1 + end_idx];
                groups.push(name.to_string());
                rest = &rest[2 + end_idx..];
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if groups.is_empty() {
        return None;
    }

    // Sort for canonical comparison
    groups.sort();
    Some(groups.join(","))
}

impl Cop for DuplicatedGroup {
    fn name(&self) -> &'static str {
        "Bundler/DuplicatedGroup"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemfile", "**/Gemfile", "**/gems.rb"]
    }

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        // Maps group key -> first occurrence line (1-indexed)
        let mut seen: HashMap<String, usize> = HashMap::new();

        for (i, line) in source.lines().enumerate() {
            let line_str = std::str::from_utf8(line).unwrap_or("");
            let line_num = i + 1;

            if let Some(group_key) = extract_group_key(line_str) {
                if let Some(&first_line) = seen.get(&group_key) {
                    let display = format_group_display(line_str);
                    diagnostics.push(self.diagnostic(
                        source,
                        line_num,
                        0,
                        format!(
                            "Gem group `{}` already defined on line {} of the Gemfile.",
                            display, first_line
                        ),
                    ));
                } else {
                    seen.insert(group_key, line_num);
                }
            }
        }
        diagnostics
    }
}

/// Extract the group display text from the line for the error message.
/// For `group :development do` returns `:development`.
/// For `group :development, :test do` returns `:development, :test`.
fn format_group_display(line: &str) -> String {
    let trimmed = line.trim();
    let start = if trimmed.starts_with("group(") {
        6
    } else {
        6
    };
    let end = trimmed.rfind(" do").unwrap_or(trimmed.len());
    let args = &trimmed[start..end];
    // Strip keyword args for display
    let parts: Vec<&str> = args
        .split(',')
        .map(|p| p.trim())
        .filter(|p| p.starts_with(':') || p.starts_with('\'') || p.starts_with('"'))
        .collect();
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DuplicatedGroup, "cops/bundler/duplicated_group");
}
