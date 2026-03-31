use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use regex::Regex;

/// FN fix: rewrote detection to match RuboCop's `notice_found?` algorithm.
///
/// Previously nitrocop searched ALL lines in the file for the copyright
/// notice, while RuboCop only examines leading consecutive comment tokens
/// (concatenated without newlines for `#` comments). This caused ~2108 FN
/// where files had the copyright after a decoration line (`#---...`) or
/// `# frozen_string_literal: true` — RuboCop's `^`-anchored default
/// pattern failed to match the concatenation, but nitrocop's per-line
/// search found it.
///
/// The new algorithm:
/// 1. Only scans leading comment lines (skips blank lines, stops at code)
/// 2. Strips `# ` from line comments and concatenates without newlines
/// 3. Preserves newlines for `=begin`/`=end` block comment content
/// 4. Uses `(?m)` so `^` matches line starts (Ruby default behavior)
///
/// FP fix: skip detection when the file has parse errors that make
/// RuboCop's `valid_syntax?` return false (AST is nil, cops skipped).
///
/// In CRuby 4.0/Prism, only `retry` (outside rescue) and `return in
/// class/module body` errors cause `valid_syntax?=false` — other semantic
/// errors (break, next, redo, yield) still produce a valid AST and cops
/// run normally. The nitrocop linter classifies all of these as "semantic"
/// and keeps running cops, so this cop must bail out explicitly for the
/// specific errors that make RuboCop skip.
///
/// Moved from `check_lines` to `check_source` to access
/// `parse_result.errors()` for this check.
pub struct Copyright;

impl Cop for Copyright {
    fn name(&self) -> &'static str {
        "Style/Copyright"
    }

    fn default_enabled(&self) -> bool {
        false // Matches vendor config/default.yml: Enabled: false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // RuboCop skips all non-Lint cops when valid_syntax? is false (ast is nil).
        // In CRuby 4.0/Prism, only `retry` and `return in class/module body` errors
        // cause this — other semantic errors (break, next, redo, yield) still produce
        // a valid AST. The linter classifies all of these as "semantic" and keeps
        // running cops, so we must bail out here for the specific errors that make
        // RuboCop skip.
        let has_fatal_semantic_error = parse_result.errors().any(|err| {
            let msg = err.message();
            msg.starts_with("Invalid retry")
                || msg.starts_with("Invalid return in class/module body")
        });
        if has_fatal_semantic_error {
            return;
        }

        let notice_pattern = config.get_str("Notice", r"^Copyright (\(c\) )?2[0-9]{3} .+");
        let autocorrect_notice = config.get_str("AutocorrectNotice", "");

        // RuboCop raises a Warning exception in verify_autocorrect_notice! when
        // AutocorrectNotice is empty, which prevents any offense from being added.
        // Match that behavior: no offenses when AutocorrectNotice is not configured.
        if autocorrect_notice.is_empty() {
            return;
        }

        // Ruby's ^ always matches at line starts; Rust's ^ only matches
        // string start by default. Prepend (?m) to match Ruby behavior.
        // This matters for =begin/=end block comments where content lines
        // are concatenated with newlines.
        let pattern_multiline = format!("(?m){}", notice_pattern);
        let regex = match Regex::new(&pattern_multiline) {
            Ok(r) => r,
            Err(_) => return,
        };

        // Match RuboCop's notice_found? behavior: only check leading consecutive
        // comment tokens. Line comments (#) are stripped of "# " and concatenated
        // without newlines. Block comment (=begin/=end) content preserves line
        // boundaries with newlines (matching Ruby token text behavior).
        // Blank lines are skipped (they don't produce tokens in Ruby's lexer).
        // The scan stops at the first non-comment, non-blank line.
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut multiline_notice = String::new();
        let mut in_block_comment = false;

        for line in &lines {
            let line_str = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => break,
            };
            let trimmed = line_str.trim();

            if trimmed.is_empty() {
                continue;
            }

            if in_block_comment {
                if trimmed.starts_with("=end") {
                    // RuboCop includes =end token text in the concatenation
                    multiline_notice.push_str(line_str);
                    multiline_notice.push('\n');
                    in_block_comment = false;
                } else {
                    // Block comment content: preserve raw text with newline
                    // (matching RuboCop's embdoc token text behavior)
                    multiline_notice.push_str(line_str);
                    multiline_notice.push('\n');
                    if regex.is_match(line_str) {
                        break;
                    }
                }
                continue;
            }

            if trimmed.starts_with("=begin") {
                // RuboCop includes =begin token text in the concatenation
                multiline_notice.push_str(line_str);
                multiline_notice.push('\n');
                in_block_comment = true;
                continue;
            }

            if let Some(after_hash) = trimmed.strip_prefix('#') {
                // RuboCop: token.text.sub(/\A# */, '') — strip first '#' then leading spaces
                let comment_content = after_hash.trim_start_matches(' ');
                multiline_notice.push_str(comment_content);

                // Early exit like RuboCop: break if notice_regexp.match?(token.text)
                if regex.is_match(trimmed) {
                    break;
                }
                continue;
            }

            // Non-comment, non-blank line = code → stop scanning
            break;
        }

        if regex.is_match(&multiline_notice) {
            return;
        }

        // No copyright notice found
        diagnostics.push(self.diagnostic(
            source,
            1,
            0,
            format!(
                "Include a copyright notice matching `{}` before any code.",
                notice_pattern
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::CopConfig;
    use std::collections::HashMap;

    /// Build a CopConfig with a non-empty AutocorrectNotice so the cop actually runs.
    /// RuboCop requires this to be set; with an empty value the cop silently skips.
    fn config_with_autocorrect_notice() -> CopConfig {
        CopConfig {
            options: HashMap::from([(
                "AutocorrectNotice".to_string(),
                serde_yml::Value::String("# Copyright (c) 2024 Acme Inc.".to_string()),
            )]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn missing_notice() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/offense/missing_notice.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn missing_notice_with_code() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/offense/missing_notice_with_code.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn missing_notice_wrong_text() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/offense/missing_notice_wrong_text.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn copyright_after_decoration() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/offense/copyright_after_decoration.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn copyright_after_frozen_string() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/offense/copyright_after_frozen_string.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn copyright_after_code() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/offense/copyright_after_code.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &Copyright,
            include_bytes!("../../../tests/fixtures/cops/style/copyright/no_offense.rb"),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn no_offense_block_comment() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/no_offense_block_comment.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn no_offense_syntax_error() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &Copyright,
            include_bytes!(
                "../../../tests/fixtures/cops/style/copyright/no_offense_syntax_error.rb"
            ),
            config_with_autocorrect_notice(),
        );
    }

    #[test]
    fn empty_autocorrect_notice_produces_no_offenses() {
        // When AutocorrectNotice is empty (the default), RuboCop raises a Warning
        // in verify_autocorrect_notice! which prevents any offense. We match that
        // behavior by returning early with no diagnostics.
        let diagnostics = crate::testutil::run_cop_full_with_config(
            &Copyright,
            b"# no copyright here\nclass Foo; end\n",
            CopConfig::default(),
        );
        assert!(
            diagnostics.is_empty(),
            "Expected no offenses with empty AutocorrectNotice, got: {:?}",
            diagnostics,
        );
    }
}
