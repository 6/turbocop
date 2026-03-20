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
        let lines: Vec<&[u8]> = source.lines().collect();
        for (line_idx, line) in lines.iter().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let trimmed = line_str.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            if let Some(pos) = line_str.find(".add_development_dependency") {
                let after_method = &line_str[pos + ".add_development_dependency".len()..];
                // If the line has an unclosed paren, join continuation lines
                let joined;
                let effective_after = if has_unclosed_paren(after_method) {
                    joined = join_continuation_lines(after_method, &lines, line_idx);
                    joined.as_str()
                } else {
                    after_method
                };
                // Only flag when the first argument is a string literal (quoted).
                // Dynamic args like `dep.name` or bare variables should be skipped,
                // matching RuboCop's `(send _ :add_development_dependency (str ...) ...)`
                if !has_string_literal_arg(effective_after) {
                    continue;
                }
                // RuboCop's NodePattern is (send _ :add_development_dependency (str ...) _? _?)
                // which matches at most 3 total arguments (gem name + up to 2 version constraints).
                // Skip lines with more than 3 args to avoid false positives.
                if count_top_level_args(effective_after) > 3 {
                    continue;
                }
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

/// Check if a string has an unclosed parenthesis (more opens than closes).
fn has_unclosed_paren(s: &str) -> bool {
    let mut depth: i32 = 0;
    let bytes = s.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() {
        match bytes[pos] {
            b'\'' | b'"' => {
                let quote = bytes[pos];
                pos += 1;
                while pos < bytes.len() && bytes[pos] != quote {
                    pos += 1;
                }
                if pos < bytes.len() {
                    pos += 1;
                }
            }
            b'(' => {
                depth += 1;
                pos += 1;
            }
            b')' => {
                depth -= 1;
                pos += 1;
            }
            _ => pos += 1,
        }
    }
    depth > 0
}

/// Join continuation lines until parens are balanced.
fn join_continuation_lines(after: &str, lines: &[&[u8]], current_idx: usize) -> String {
    let mut result = after.to_string();
    let mut depth: i32 = 0;
    for &b in after.as_bytes() {
        match b {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
    }
    if depth <= 0 {
        return result;
    }
    for line in lines.iter().skip(current_idx + 1) {
        if let Ok(s) = std::str::from_utf8(line) {
            result.push(' ');
            result.push_str(s.trim());
            for &b in s.as_bytes() {
                match b {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
            }
            if depth <= 0 {
                break;
            }
        }
    }
    result
}

/// Check if the first argument after the method call is a string literal.
/// Recognizes standard quotes ('...', "...") and percent string literals
/// (%q<...>, %Q(...), %[...], etc.) which parse to `(str ...)` in RuboCop's AST.
/// Excludes `.freeze` suffixed strings which are `(send (str ...) :freeze)` in AST,
/// not bare `(str ...)` nodes, so RuboCop's NodePattern doesn't match them.
fn has_string_literal_arg(after_method: &str) -> bool {
    let trimmed = after_method.trim_start();
    let trimmed = if let Some(stripped) = trimmed.strip_prefix('(') {
        stripped.trim_start()
    } else {
        trimmed
    };
    if trimmed.starts_with('\'') || trimmed.starts_with('"') {
        let quote = trimmed.as_bytes()[0];
        // Find end of string literal and check for .freeze
        if let Some(end) = trimmed[1..].find(|c: char| c as u8 == quote) {
            let after_string = &trimmed[end + 2..];
            if after_string.starts_with(".freeze") {
                return false;
            }
        }
        return true;
    }
    if is_percent_string(trimmed) {
        return !has_freeze_suffix(trimmed);
    }
    false
}

/// Check if the string starts with a Ruby percent string literal.
/// Matches: %q<...>, %Q<...>, %<...>, %(, %[, %{, etc.
fn is_percent_string(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'%') {
        return false;
    }
    if bytes.len() < 2 {
        return false;
    }
    let next = match bytes[1] {
        b'q' | b'Q' => {
            if bytes.len() < 3 {
                return false;
            }
            bytes[2]
        }
        other => other,
    };
    matches!(next, b'<' | b'(' | b'[' | b'{')
}

/// Check if a percent string literal has a `.freeze` suffix.
/// E.g., `%q<rails>.freeze` -> true, `%q<rails>` -> false.
fn has_freeze_suffix(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'%') || bytes.len() < 3 {
        return false;
    }
    let start = match bytes[1] {
        b'q' | b'Q' => 3,
        _ => 2,
    };
    if start > bytes.len() {
        return false;
    }
    let opener = bytes[start - 1];
    let closer = match opener {
        b'<' => b'>',
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        _ => return false,
    };
    // Find the closing delimiter
    if let Some(end) = s[start..].find(|c: char| c as u8 == closer) {
        let after = &s[start + end + 1..];
        after.starts_with(".freeze")
    } else {
        false
    }
}

/// Count top-level arguments in a method call (commas not inside brackets/parens).
/// Returns the number of arguments (1 for a single arg, 2 for two, etc.).
fn count_top_level_args(after_method: &str) -> usize {
    let trimmed = after_method.trim_start();
    let content = if let Some(stripped) = trimmed.strip_prefix('(') {
        stripped
    } else {
        trimmed
    };
    let mut depth = 0usize;
    let mut count = 1;
    for ch in content.chars() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            ',' if depth == 0 => count += 1,
            '\n' => break,
            _ => {}
        }
    }
    count
}

/// Extract the content of a percent string literal (e.g., `%q<erubis>` -> `erubis`).
fn extract_percent_string_content(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'%') || bytes.len() < 3 {
        return None;
    }
    let start = match bytes[1] {
        b'q' | b'Q' => 3,
        _ => 2,
    };
    if start > bytes.len() {
        return None;
    }
    let opener = bytes[start - 1];
    let closer = match opener {
        b'<' => b'>',
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        _ => return None,
    };
    let content = &s[start..];
    content
        .find(|c: char| c as u8 == closer)
        .map(|end| &content[..end])
}

/// Check if the gem name following the method call is in the allowed list.
fn is_gem_allowed(after_method: &str, allowed_gems: &[String]) -> bool {
    if allowed_gems.is_empty() {
        return false;
    }
    // Try to extract gem name from patterns like:
    //   ('gem_name', ...) or  'gem_name' or "gem_name"
    let trimmed = after_method.trim_start();
    let trimmed = if let Some(stripped) = trimmed.strip_prefix('(') {
        stripped.trim_start()
    } else {
        trimmed
    };
    let gem_name = if trimmed.starts_with('\'') || trimmed.starts_with('"') {
        let quote = trimmed.as_bytes()[0];
        let rest = &trimmed[1..];
        rest.find(|c: char| c as u8 == quote)
            .map(|end| &rest[..end])
    } else if is_percent_string(trimmed) {
        extract_percent_string_content(trimmed)
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
