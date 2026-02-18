use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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

    fn check_lines(&self, source: &SourceFile, _config: &CopConfig) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Find `# rubocop:` directive — must be the first `#` that starts the directive
            // Ignore lines where `# rubocop:` is commented out (e.g., `# # rubocop:disable`)
            // or quoted (e.g., `# "rubocop:disable"`)
            let Some(hash_pos) = find_directive_start(line_str) else {
                continue;
            };

            let directive_text = &line_str[hash_pos..];
            let after_hash = directive_text[1..].trim_start();

            // Must start with `rubocop:` (not `"rubocop:` or `# rubocop:`)
            if !after_hash.starts_with("rubocop:") {
                continue;
            }

            let after_rubocop_colon = &after_hash["rubocop:".len()..];

            // Check if mode name is missing
            if after_rubocop_colon.is_empty() || after_rubocop_colon.trim().is_empty() {
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    hash_pos,
                    "Malformed directive comment detected. The mode name is missing.".to_string(),
                ));
                continue;
            }

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
                continue;
            }

            // After the mode, extract the rest (cop names + optional comment)
            let after_mode = &after_rubocop_colon[mode_end..].trim_start();

            // Check if cop name is missing
            if after_mode.is_empty() {
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    hash_pos,
                    "Malformed directive comment detected. The cop name is missing.".to_string(),
                ));
                continue;
            }

            // Validate the cop list format:
            // - Cop names must be separated by commas
            // - Comments must start with `--`
            // - No duplicate `# rubocop:` directives in one line
            if is_malformed_cop_list(after_mode) {
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    hash_pos,
                    "Malformed directive comment detected. Cop names must be separated by commas. Comment in the directive must start with `--`.".to_string(),
                ));
            }
        }

        diagnostics
    }
}

/// Find the position of the `#` that starts a rubocop directive.
/// Returns None if there's no directive, or if the directive is commented out
/// (e.g., `# # rubocop:disable`) or quoted.
fn find_directive_start(line: &str) -> Option<usize> {
    // Find `# rubocop:` — possibly after code (inline directive)
    let mut search_from = 0;
    loop {
        let rest = &line[search_from..];
        let hash_pos = rest.find('#')?;
        let abs_pos = search_from + hash_pos;

        let after_hash = &rest[hash_pos + 1..].trim_start();

        if after_hash.starts_with("rubocop:") {
            // Check it's not a commented-out directive (another # before this one on the same effective comment)
            // If there's a `#` before this position in a comment context, skip
            let before = &line[..abs_pos];
            let before_trimmed = before.trim();
            if before_trimmed.ends_with('#') {
                // This is a `# # rubocop:` pattern — skip
                search_from = abs_pos + 1;
                continue;
            }
            // Check not quoted
            if before_trimmed.ends_with('"') || before_trimmed.ends_with('\'') {
                search_from = abs_pos + 1;
                continue;
            }
            return Some(abs_pos);
        }

        search_from = abs_pos + 1;
    }
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

    // Check for duplicate `# rubocop:` within the remaining text
    if cop_part.contains("# rubocop:") || cop_part.contains("#rubocop:") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CopDirectiveSyntax, "cops/lint/cop_directive_syntax");
}
