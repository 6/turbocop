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
///
/// ## Investigation notes (2026-03-30)
///
/// 85 FNs + 1 FP remaining: The `^` anchor prevented matching inline
/// directives embedded in longer comments (e.g., `# some text # rubocop:disable Foo`
/// or `#: type_annotation # rubocop:disable Style/RedundantSelf`). Prism
/// treats the whole line comment as one node, so the directive is not at
/// position 0 of the comment bytes.
///
/// Fix: Removed the `^` anchor and instead use a strict cop-name pattern
/// (`[A-Za-z]\w+(/[A-Za-z]\w+)*` or `all`) so that YARD prose like
/// `# rubocop:enable ...` doesn't match (since `...` isn't a valid cop name).
/// Also skip `disable all` and `todo all` directives because they suppress
/// all cops including this one — RuboCop never reports an offense for them.
pub struct DisableCopsWithinSourceCodeDirective;

/// Regex matching rubocop directive comments with flexible whitespace,
/// mirroring RuboCop's `DirectiveComment::DIRECTIVE_COMMENT_REGEXP`.
/// Not anchored to `^` so it matches inline directives embedded in longer
/// comments (e.g. `# some text # rubocop:disable Foo`). Uses a strict
/// cop-name pattern instead to avoid matching YARD prose that mentions
/// directives (e.g. `# rubocop:enable ...` where `...` is not a cop name).
/// Captures: (1) mode (disable/enable/todo), (2) cop list.
static DIRECTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"#\s*rubocop\s*:\s*(disable|enable|todo)\s+(all\b|[A-Za-z]\w+(?:/[A-Za-z]\w+)*(?:\s*,\s*[A-Za-z]\w+(?:/[A-Za-z]\w+)*)*)",
    )
    .unwrap()
});

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

            let mode = &caps[1];
            let cop_list_raw = &caps[2];

            // `# rubocop:disable all` and `# rubocop:todo all` suppress all
            // cops including this one, so RuboCop never reports an offense for
            // them.  Skip to avoid FPs.
            if (mode == "disable" || mode == "todo") && cop_list_raw.trim() == "all" {
                continue;
            }

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
