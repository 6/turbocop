use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10, updated 2026-03-14)
///
/// Corpus oracle reported FP=8, FN=13 prior to comment-skipping fix.
///
/// Root cause: nitrocop checked the first non-empty line, but RuboCop checks
/// the first non-comment *token*. This means:
/// - FP: Files starting with indented comments (e.g. `  # frozen_string_literal: true`)
///   were flagged even though RuboCop skips comment tokens entirely.
/// - FN: Files starting with unindented comments followed by indented code were
///   missed because nitrocop stopped at the comment line (column 0).
///
/// Fix: skip lines that are pure comments (trimmed content starts with `#`)
/// before checking indentation, matching RuboCop's `first_token` logic that
/// filters out comment tokens. Also handle UTF-8 BOM.
///
/// Additionally fixed message text to match RuboCop exactly:
/// "Indentation of first line in file detected." (was missing "in file").
///
/// ## Whitespace-only blank line fix (2026-03-15)
///
/// FP=2 on jruby and webistrano: lines containing only whitespace (including
/// CRLF blank lines that become `" \r"` after `SourceFile::lines()`) were
/// treated as non-blank and flagged as initial indentation. RuboCop skips
/// whitespace-only lines. Fix: when scanning for the first non-blank
/// non-comment line, also treat `\r` as ignorable horizontal whitespace.
/// This also fixes the paired FN where the actual indented code line after the
/// whitespace-only blank line was never reached.
pub struct InitialIndentation;

impl Cop for InitialIndentation {
    fn name(&self) -> &'static str {
        "Layout/InitialIndentation"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // RuboCop checks the first non-comment token's column. We approximate
        // this by skipping blank lines and lines whose first non-whitespace
        // character is `#` (pure comment lines). This matches RuboCop's
        // `first_token` which filters `!t.text.start_with?('#')`.
        for (i, line) in source.lines().enumerate() {
            if line.is_empty() {
                continue;
            }

            // Skip UTF-8 BOM (EF BB BF) at the start of the file
            let effective = if i == 0 && line.starts_with(&[0xEF, 0xBB, 0xBF]) {
                &line[3..]
            } else {
                line
            };

            // Skip whitespace-only lines (treat as blank) and pure comment lines
            let trimmed = effective.iter().find(|&&b| !is_horizontal_whitespace(b));
            match trimmed {
                None => continue,        // whitespace-only line
                Some(&b'#') => continue, // pure comment line
                _ => {}
            }

            // Now we have the first non-empty, non-comment line
            if effective.first() == Some(&b' ') || effective.first() == Some(&b'\t') {
                let ws_len = effective
                    .iter()
                    .take_while(|&&b| b == b' ' || b == b'\t')
                    .count();
                // Calculate the actual byte offset accounting for BOM
                let bom_offset = if i == 0 && line.starts_with(&[0xEF, 0xBB, 0xBF]) {
                    3
                } else {
                    0
                };
                let mut diag = self.diagnostic(
                    source,
                    i + 1,
                    bom_offset,
                    "Indentation of first line in file detected.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    if let Some(start) = source.line_col_to_offset(i + 1, 0) {
                        corr.push(crate::correction::Correction {
                            start: start + bom_offset,
                            end: start + bom_offset + ws_len,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                }
                diagnostics.push(diag);
            }
            break;
        }
    }
}

fn is_horizontal_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::source::SourceFile;

    crate::cop_scenario_fixture_tests!(
        InitialIndentation,
        "cops/layout/initial_indentation",
        space_indent = "space_indent.rb",
        tab_indent = "tab_indent.rb",
        deep_indent = "deep_indent.rb",
        comment_then_indented_code = "comment_then_indented_code.rb",
        comments_then_indented = "comments_then_indented.rb",
        whitespace_only_blank_then_indented = "whitespace_only_blank_then_indented.rb",
    );

    #[test]
    fn leading_blank_then_indented() {
        let source = SourceFile::from_bytes("test.rb", b"\n  x = 1\n".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
    }

    #[test]
    fn leading_blank_then_unindented() {
        let source = SourceFile::from_bytes("test.rb", b"\nx = 1\n".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn whitespace_only_crlf_line_then_unindented() {
        let source = SourceFile::from_bytes("test.rb", b"# comment\r\n \r\nx = 1\r\n".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    #[test]
    fn autocorrect_remove_spaces() {
        let input = b"  x = 1\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&InitialIndentation, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }

    #[test]
    fn autocorrect_remove_tabs() {
        let input = b"\tx = 1\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&InitialIndentation, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n");
    }

    #[test]
    fn empty_file() {
        let source = SourceFile::from_bytes("test.rb", b"".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty());
    }

    // FP pattern: indented comment followed by indented code should flag the CODE line, not the comment
    #[test]
    fn indented_comment_then_indented_code() {
        // e.g. rails destroy_async_parent.rb: " # frozen_string_literal: true\n\n class Foo\n"
        let source = SourceFile::from_bytes(
            "test.rb",
            b" # frozen_string_literal: true\n\n class Foo\nend\n".to_vec(),
        );
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1, "should flag indented code, not comment");
        assert_eq!(
            diags[0].location.line, 3,
            "should flag line 3 (the code line)"
        );
    }

    // FP pattern: indented comment followed by code at column 0 → no offense
    #[test]
    fn indented_comment_then_unindented_code() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b" # frozen_string_literal: true\nclass Foo\nend\n".to_vec(),
        );
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "should not flag when code starts at column 0"
        );
    }

    // FP pattern: deeply indented comment followed by code at column 0 → no offense
    #[test]
    fn deeply_indented_comment_then_code() {
        // e.g. pry example_nesting.rb: "                                     # []\nclass A\n"
        let source = SourceFile::from_bytes(
            "test.rb",
            b"                                     # []\nclass A\nend\n".to_vec(),
        );
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "should not flag when code starts at column 0"
        );
    }

    // FP pattern: tab-indented comment → no offense when code is at column 0
    #[test]
    fn tab_indented_comment_then_code() {
        // e.g. WhatWeb pyro-cms.rb: "\t##\n# comment\nPlugin.define do\n"
        let source = SourceFile::from_bytes(
            "test.rb",
            b"\t##\n# comment\nPlugin.define do\nend\n".to_vec(),
        );
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "should not flag when code starts at column 0"
        );
    }

    // FN pattern: comments then indented code at line 3
    #[test]
    fn comment_blank_then_indented_code() {
        // e.g. rufo spec files: "#~# ORIGINAL\n\n foo  and  bar\n"
        let source =
            SourceFile::from_bytes("test.rb", b"#~# ORIGINAL\n\n foo  and  bar\n".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1, "should flag indented code after comments");
        assert_eq!(diags[0].location.line, 3);
    }

    // FN pattern: shebang + comments then indented code
    #[test]
    fn shebang_comments_then_indented_code() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"#!/usr/bin/env ruby\n# comment\n\n  x = 1\n".to_vec(),
        );
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "should flag indented code after shebang and comments"
        );
        assert_eq!(diags[0].location.line, 4);
    }

    // FP fix: blank line with whitespace before unindented code → no offense
    // RuboCop treats whitespace-only lines as blank (skips them).
    #[test]
    fn whitespace_only_blank_line_then_unindented_code() {
        // e.g. jruby sumcol.ruby-2.ruby: comments, then " \n", then "puts ..."
        // The " \n" line should be skipped, and "puts" at column 0 → no offense
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# comment\n# comment2\n# comment3\n \nputs 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "should not flag whitespace-only blank line; first code line is at column 0"
        );
    }

    // FN fix: blank line with whitespace before indented code → offense on code line
    #[test]
    fn whitespace_only_blank_line_then_indented_code() {
        // e.g. jruby sumcol.ruby-2.ruby: comments, then " \n", then " puts ..."
        // The " \n" line should be skipped, " puts" is indented → offense
        let source = SourceFile::from_bytes(
            "test.rb",
            b"# comment\n# comment2\n# comment3\n \n puts 'hello'\n".to_vec(),
        );
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "should flag indented code after whitespace-only blank line"
        );
        assert_eq!(diags[0].location.line, 5, "offense should be on line 5");
    }

    // No FP: file with only comments (all indented) → no offense
    #[test]
    fn only_indented_comments() {
        let source = SourceFile::from_bytes("test.rb", b"  # comment 1\n  # comment 2\n".to_vec());
        let mut diags = Vec::new();
        InitialIndentation.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(diags.is_empty(), "should not flag comment-only files");
    }
}
