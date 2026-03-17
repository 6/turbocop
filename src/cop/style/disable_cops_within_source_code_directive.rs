use std::sync::LazyLock;

use regex::Regex;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects `# rubocop:disable`, `# rubocop:enable`, and `# rubocop:todo`
/// directives in source code comments.
///
/// ## Investigation notes (2026-03-15)
///
/// Root cause of FPs: The original line-based `check_lines` implementation
/// scanned raw source lines, which picked up directive-like text embedded
/// inside string literals (heredocs, quoted strings). RuboCop only checks
/// actual parser comments via `processed_source.comments`, so it correctly
/// ignores directives inside strings.
///
/// Root cause of FNs: The original implementation used exact string matching
/// (`"# rubocop:disable "`) requiring exactly one space after `#` and a
/// trailing space after the mode keyword. RuboCop's `DirectiveComment` uses
/// a regex that allows flexible whitespace: `#\s*rubocop\s*:\s*(disable|enable|todo)`.
///
/// Fix: Switched from `check_lines` to `check_source`, iterating over
/// `parse_result.comments()` (Prism's AST-derived comment list) and using
/// a regex matching RuboCop's flexible spacing. Also fixed per-cop offense
/// emission with `AllowedCops`: RuboCop emits one offense per comment joining
/// all disallowed cop names, not one offense per disallowed cop.
///
/// ## Investigation notes (2026-03-17)
///
/// 5 remaining FPs: The directive regex was not anchored, so it matched
/// directive-like text embedded inside YARD documentation comments (e.g.,
/// `# Checks that \`# rubocop:enable ...\` and \`# rubocop:disable ...\``).
/// Prism's comment bytes always start with `#`, so anchoring the regex with
/// `^` ensures only actual directives at the start of the comment are matched.
/// All 5 FPs were from rubocop's own source (YARD docs) and a shoryuken spec.
pub struct DisableCopsWithinSourceCodeDirective;

/// Regex matching rubocop directive comments with flexible whitespace,
/// mirroring RuboCop's `DirectiveComment::DIRECTIVE_COMMENT_REGEXP`.
/// Anchored to `^` since Prism comment bytes always start with `#`,
/// preventing matches against directive-like text embedded in prose.
/// Captures: (1) mode (disable/enable/todo), (2) cop list.
static DIRECTIVE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#\s*rubocop\s*:\s*(disable|enable|todo)\s+(.+)").unwrap());

impl Cop for DisableCopsWithinSourceCodeDirective {
    fn name(&self) -> &'static str {
        "Style/DisableCopsWithinSourceCodeDirective"
    }

    fn default_enabled(&self) -> bool {
        false
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
        let allowed_cops = config.get_string_array("AllowedCops").unwrap_or_default();

        for comment in parse_result.comments() {
            let loc = comment.location();
            let comment_bytes = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            let Ok(comment_str) = std::str::from_utf8(comment_bytes) else {
                continue;
            };

            let Some(caps) = DIRECTIVE_RE.captures(comment_str) else {
                continue;
            };

            let cop_list_raw = &caps[2];

            // Strip trailing comment marker (-- reason)
            let cop_list = match cop_list_raw.find("--") {
                Some(idx) => &cop_list_raw[..idx],
                None => cop_list_raw,
            };

            let cop_names: Vec<&str> = cop_list
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            let (line, col) = source.offset_to_line_col(loc.start_offset());

            if !allowed_cops.is_empty() {
                let disallowed: Vec<&str> = cop_names
                    .iter()
                    .copied()
                    .filter(|c| !allowed_cops.iter().any(|a| a == c))
                    .collect();

                if disallowed.is_empty() {
                    continue;
                }

                // RuboCop emits one offense per comment, joining all disallowed cop names
                let cops_formatted: Vec<String> =
                    disallowed.iter().map(|c| format!("`{}`", c)).collect();
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    col,
                    format!(
                        "RuboCop disable/enable directives for {} are not permitted.",
                        cops_formatted.join(", ")
                    ),
                ));
            } else {
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    col,
                    "RuboCop disable/enable directives are not permitted.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        DisableCopsWithinSourceCodeDirective,
        "cops/style/disable_cops_within_source_code_directive"
    );
}
