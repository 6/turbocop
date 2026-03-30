use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Checks that `# rubocop:` directives are strictly formatted.
///
/// ## Investigation findings (2026-03-08)
///
/// **Root cause of 277 FPs:** Two bugs:
///
/// 1. **Space after colon** (`# rubocop: disable`): RuboCop's `DIRECTIVE_MARKER_REGEXP` uses
///    `#\s*rubocop\s*:\s*` which allows optional whitespace around the colon. Our code
///    used `strip_prefix("rubocop:")` and then extracted the mode from `after_rubocop_colon`
///    without trimming leading whitespace. When `after_rubocop_colon` was `" disable ..."`,
///    `mode_end` was 0 (first char is space), making `mode` an empty string that didn't
///    match any valid mode. Fixed by trimming `after_rubocop_colon` before mode extraction.
///
/// 2. **push/pop without cop names**: RuboCop's `missing_cop_name?` explicitly returns false
///    for push/pop modes. Our code flagged them as "missing cop name". Fixed by skipping
///    the cop name check for push/pop modes.
///
/// Also added support for `rubocop\s*:` (space before colon) to match RuboCop's regex,
/// though this is extremely rare in practice.
///
/// ## Investigation findings (2026-03-19)
///
/// **Root cause of 46 FNs:** The cop only checked for `:` as a department/cop separator
/// (e.g., `Layout:LineLength`) but missed many other malformed cop name patterns found
/// in the corpus:
///   - `Rails::SkipsModelValidations` (double colon separator)
///   - `Rails/SkipsModelValidations:` (trailing colon)
///   - `Metrics/BlockLength(RuboCop)` (parenthetical text)
///   - `Rails/FindEach.` (trailing period)
///   - `Naming/PredicatePrefix?` (trailing question mark)
///   - `/BlockLength` or `Metrics/` (leading/trailing slash)
///   - `Discourse/NoChdir because ...` (comment without `--` prefix, caught by multi-word check)
///
/// Fixed by replacing `has_colon_separator` with `has_invalid_cop_name` that validates
/// each cop name token contains only `[A-Za-z0-9/_]` and doesn't start/end with `/`.
///
/// ## Corpus investigation (2026-03-20)
///
/// FN=1: `Dir.chdir("#{__dir__}/..") # rubocop:disable Discourse/NoChdir because ...`
/// `find_directive_start` treated `#` in `"#{__dir__}"` as a comment start, setting
/// `first_hash_seen = true`, which caused the real directive to be rejected. Fix:
/// skip `#` followed by `{` (string interpolation) when setting `first_hash_seen`.
///
/// ## Corpus investigation (2026-03-30)
///
/// FN=1: `# rubocop:disable Layout/LineLength,` (trailing comma after cop name).
/// `is_malformed_cop_list` split by comma, got `["Layout/LineLength", ""]`, and skipped
/// the empty trailing element. Fix: detect trailing comma in `cop_part` before splitting.
pub struct CopDirectiveSyntax;

impl Cop for CopDirectiveSyntax {
    fn name(&self) -> &'static str {
        "Lint/CopDirectiveSyntax"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut byte_offset = 0usize;
        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => {
                    byte_offset += line.len() + 1;
                    continue;
                }
            };

            // Find `# rubocop:` directive — must be the first `#` that starts the directive
            // Ignore lines where `# rubocop:` is commented out (e.g., `# # rubocop:disable`)
            // or quoted (e.g., `# "rubocop:disable"`)
            let Some(hash_pos) = find_directive_start(line_str) else {
                byte_offset += line.len() + 1;
                continue;
            };

            // Skip directives inside strings/heredocs
            if !code_map.is_not_string(byte_offset + hash_pos) {
                byte_offset += line.len() + 1;
                continue;
            }

            let directive_text = &line_str[hash_pos..];
            let after_hash = directive_text[1..].trim_start();

            // Must start with `rubocop:` (not `"rubocop:` or `# rubocop:`)
            if let Some(after_rubocop_colon) = strip_rubocop_prefix(after_hash) {
                // Trim leading whitespace (RuboCop allows `# rubocop: disable` with space after colon)
                let after_rubocop_colon = after_rubocop_colon.trim_start();

                // Check if mode name is missing
                if after_rubocop_colon.is_empty() {
                    diagnostics.push(
                        self.diagnostic(
                            source,
                            i + 1,
                            hash_pos,
                            "Malformed directive comment detected. The mode name is missing."
                                .to_string(),
                        ),
                    );
                } else {
                    // Extract mode name (first word after `rubocop:`)
                    let mode_end = after_rubocop_colon
                        .find(|c: char| c.is_ascii_whitespace())
                        .unwrap_or(after_rubocop_colon.len());
                    let mode = &after_rubocop_colon[..mode_end];

                    // Validate mode
                    if !matches!(mode, "enable" | "disable" | "todo" | "push" | "pop") {
                        diagnostics.push(self.diagnostic(
                            source,
                            i + 1,
                            hash_pos,
                            "Malformed directive comment detected. The mode name must be one of `enable`, `disable`, `todo`, `push`, or `pop`.".to_string(),
                        ));
                    } else {
                        // After the mode, extract the rest (cop names + optional comment)
                        let after_mode = &after_rubocop_colon[mode_end..].trim_start();

                        // push/pop without cop names is valid (RuboCop allows bare push/pop)
                        let is_push_pop = matches!(mode, "push" | "pop");

                        // Check if cop name is missing (except for push/pop)
                        if after_mode.is_empty() && !is_push_pop {
                            diagnostics.push(self.diagnostic(
                                source,
                                i + 1,
                                hash_pos,
                                "Malformed directive comment detected. The cop name is missing.".to_string(),
                            ));
                        } else if !after_mode.is_empty() && is_malformed_cop_list(after_mode) {
                            diagnostics.push(self.diagnostic(
                                source,
                                i + 1,
                                hash_pos,
                                "Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.".to_string(),
                            ));
                        }
                    }
                }
            }

            byte_offset += line.len() + 1;
        }
    }
}

/// Find the position of the `#` that starts a rubocop directive.
/// Returns None if there's no directive, or if the directive is commented out
/// (e.g., `# # rubocop:disable`) or quoted.
fn find_directive_start(line: &str) -> Option<usize> {
    // Find `# rubocop:` — possibly after code (inline directive)
    let mut search_from = 0;
    let mut first_hash_seen = false;
    loop {
        let rest = &line[search_from..];
        let hash_pos = rest.find('#')?;
        let abs_pos = search_from + hash_pos;

        let after_hash = &rest[hash_pos + 1..].trim_start();

        if strip_rubocop_prefix(after_hash).is_some() {
            // Check it's not a commented-out directive (another # before this one on the same effective comment)
            // If there's a `#` before this position in a comment context, skip
            let before = &line[..abs_pos];
            let before_trimmed = before.trim();
            if before_trimmed.ends_with('#') {
                // This is a `# # rubocop:` pattern — skip
                search_from = abs_pos + 1;
                continue;
            }
            // Note: No quote check needed here. Directives inside strings like
            // `"# rubocop:disable ..."` are filtered by the `code_map.is_not_string()`
            // check in the caller. The previous `ends_with('"')` check was too aggressive
            // — it rejected lines like `code") # rubocop:disable ...` where the `"` was
            // a closing string quote in code, not a quote wrapping the directive.
            // If we already saw a `#` that started a non-directive comment,
            // then this `# rubocop:` is inside comment text (e.g. documentation),
            // not an actual directive.
            if first_hash_seen {
                return None;
            }
            return Some(abs_pos);
        }

        // This `#` starts a comment but is NOT a rubocop directive.
        // Any subsequent `# rubocop:` on this line is inside the comment text.
        // Exception: `#` followed by `{` is string interpolation (e.g., "#{foo}"),
        // not a comment start — don't count it.
        let next_char = line.as_bytes().get(abs_pos + 1).copied();
        if next_char != Some(b'{') && !first_hash_seen {
            first_hash_seen = true;
        }

        search_from = abs_pos + 1;
    }
}

/// Strip the `rubocop:` prefix from a string, allowing optional whitespace before the colon.
/// This matches RuboCop's `DIRECTIVE_MARKER_REGEXP` which uses `rubocop\s*:\s*`.
/// Returns the remainder after `rubocop:` (may have leading whitespace).
fn strip_rubocop_prefix(s: &str) -> Option<&str> {
    let rest = s.strip_prefix("rubocop")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(':')?;
    Some(rest)
}

/// Check if the cop list portion is malformed.
/// A valid cop list is: `CopName1, CopName2 -- optional comment` or just `all`.
fn is_malformed_cop_list(cops_str: &str) -> bool {
    // Strip `-- comment` suffix if present
    let (cop_part, _) = match cops_str.find(" -- ") {
        Some(idx) => (&cops_str[..idx], &cops_str[idx..]),
        None => {
            // Check if it starts with `--` directly
            if cops_str.starts_with("--") {
                return false; // Just a comment, no cops — already handled by missing cop name
            }
            (cops_str, "")
        }
    };

    // A trailing comma indicates a malformed cop list (e.g., `Layout/LineLength,`)
    if cop_part.trim_end().ends_with(',') {
        return true;
    }

    // Split by comma and check each part
    let parts: Vec<&str> = cop_part.split(',').map(|s| s.trim()).collect();

    for part in &parts {
        if part.is_empty() {
            continue;
        }
        // Each part should be a single cop name (letters, digits, `/`, `_`)
        // or `all`. If it contains spaces, it means multiple cops without commas
        // or a comment without `--`.
        let words: Vec<&str> = part.split_whitespace().collect();
        if words.len() > 1 {
            // Multiple words in a single comma-separated segment — malformed
            // Could be missing comma or comment without `--`
            return true;
        }
    }

    // Check for invalid cop name tokens.
    // Valid cop names contain only letters, digits, `/`, and `_` (e.g., `Layout/LineLength`).
    // Common malformed patterns:
    //   - `:` separator: `Layout:LineLength` (should be `/`)
    //   - `::` separator: `Rails::SkipsModelValidations`
    //   - trailing colon: `Metrics/BlockLength:`
    //   - trailing punctuation: `Rails/FindEach.`, `Naming/PredicatePrefix?`
    //   - parenthetical: `Metrics/BlockLength(RuboCop)`
    //   - leading/trailing slash: `/BlockLength`, `Metrics/`
    for part in &parts {
        if part.is_empty() || *part == "all" {
            continue;
        }
        if has_invalid_cop_name(part) {
            return true;
        }
    }

    // Check for duplicate `# rubocop:` within the remaining text
    if cop_part.contains("# rubocop:") || cop_part.contains("#rubocop:") {
        return true;
    }

    false
}

/// Check if a cop name token is malformed (contains invalid characters).
/// Valid cop names match `[A-Za-z][A-Za-z0-9]*(/[A-Za-z][A-Za-z0-9]*)*` — i.e.,
/// PascalCase identifiers optionally separated by `/`.
/// Returns true if the token contains characters outside `[A-Za-z0-9/_]`,
/// or starts/ends with `/`.
fn has_invalid_cop_name(token: &str) -> bool {
    // Must not be empty (caller already checks)
    if token.is_empty() {
        return false;
    }
    // Must not start or end with `/`
    if token.starts_with('/') || token.ends_with('/') {
        return true;
    }
    // Every character must be alphanumeric, `/`, or `_`
    if token
        .chars()
        .any(|c| !c.is_ascii_alphanumeric() && c != '/' && c != '_')
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CopDirectiveSyntax, "cops/lint/cop_directive_syntax");
}
