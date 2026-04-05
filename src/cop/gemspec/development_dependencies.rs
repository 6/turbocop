use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Mirrors RuboCop's style-dependent scope for development dependencies.
///
/// `EnforcedStyle: gemspec` was previously a complete FN path: the cop returned
/// early and only included `*.gemspec`, so literal `gem "foo"` declarations in
/// `Gemfile` and `gems.rb` were never checked. The fix keeps the existing
/// line-based `add_development_dependency` detection for `Gemfile`/`gems.rb`
/// styles and adds an AST-based `gem` matcher for `gemspec` style, which
/// preserves RuboCop's literal-string-only behavior (`AllowedGems`, percent
/// strings, and `.freeze` exclusions all still line up with `(str ...)`).
pub struct DevelopmentDependencies;

impl Cop for DevelopmentDependencies {
    fn name(&self) -> &'static str {
        "Gemspec/DevelopmentDependencies"
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/*.gemspec", "**/Gemfile", "**/gems.rb"]
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
                    development_dependencies_message(style),
                ));
            }
        }
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "Gemfile");
        if style != "gemspec" {
            return;
        }

        let mut visitor = GemspecStyleGemVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            allowed_gems: config.get_string_array("AllowedGems").unwrap_or_default(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

fn development_dependencies_message(style: &str) -> String {
    if style == "gemspec" {
        "Specify development dependencies in `gemspec`.".to_string()
    } else {
        format!("Specify development dependencies in `{style}` instead of gemspec.")
    }
}

struct GemspecStyleGemVisitor<'a> {
    cop: &'a DevelopmentDependencies,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    allowed_gems: Vec<String>,
}

impl<'pr> Visit<'pr> for GemspecStyleGemVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() != b"gem" {
            ruby_prism::visit_call_node(self, node);
            return;
        }

        let Some(gem_name) = gem_name_from_call(node) else {
            ruby_prism::visit_call_node(self, node);
            return;
        };
        if self
            .allowed_gems
            .iter()
            .any(|allowed| allowed.as_bytes() == gem_name.as_slice())
        {
            ruby_prism::visit_call_node(self, node);
            return;
        }

        let loc = node.message_loc().unwrap_or(node.location());
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            development_dependencies_message("gemspec"),
        ));

        ruby_prism::visit_call_node(self, node);
    }
}

fn gem_name_from_call(call: &ruby_prism::CallNode<'_>) -> Option<Vec<u8>> {
    let first_arg = util::first_positional_arg(call)?;
    util::string_value(&first_arg)
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
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        DevelopmentDependencies,
        "cops/gemspec/development_dependencies"
    );

    fn gemspec_style_config(allowed_gems: &[&str]) -> CopConfig {
        let mut options = HashMap::new();
        options.insert(
            "EnforcedStyle".to_string(),
            serde_yml::Value::String("gemspec".to_string()),
        );
        if !allowed_gems.is_empty() {
            options.insert(
                "AllowedGems".to_string(),
                serde_yml::Value::Sequence(
                    allowed_gems
                        .iter()
                        .map(|gem| serde_yml::Value::String((*gem).to_string()))
                        .collect(),
                ),
            );
        }
        CopConfig {
            options,
            ..CopConfig::default()
        }
    }

    #[test]
    fn offense_gemfile_gemspec_style() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &DevelopmentDependencies,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/development_dependencies/offense_gemfile_gemspec_style.rb"
            ),
            gemspec_style_config(&[]),
        );
    }

    #[test]
    fn offense_gems_rb_gemspec_style() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &DevelopmentDependencies,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/development_dependencies/offense_gems_rb_gemspec_style.rb"
            ),
            gemspec_style_config(&[]),
        );
    }

    #[test]
    fn no_offense_gemspec_style() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &DevelopmentDependencies,
            include_bytes!(
                "../../../tests/fixtures/cops/gemspec/development_dependencies/no_offense_gemspec_style.rb"
            ),
            gemspec_style_config(&["allowed"]),
        );
    }
}
