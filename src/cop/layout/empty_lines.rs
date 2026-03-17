use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle reported FP=12,897, FN=23. Root cause: whitespace-only lines
/// (spaces/tabs) were treated as blank lines, but RuboCop's `EmptyLines` only
/// counts truly empty lines (zero bytes after newline removal). 91% of FPs came
/// from twilio-ruby's auto-generated code with indentation on blank lines.
/// Fix: changed `line.iter().all(|&b| b == b' ' || ...)` to `line.is_empty()`.
/// Acceptance gate after fix: expected=12,238, actual=13,320, excess=0, missing=0.
/// The 23 FNs are pre-existing (likely CodeMap edge cases) and unrelated.
///
/// ## Corpus investigation (2026-03-16)
///
/// FP=11 remained. All 11 FPs were consecutive blank lines at the very start of
/// a file (lines 1-2). Root cause: RuboCop's `each_extra_empty_line` starts with
/// `prev_line = 1` and uses `LINE_OFFSET = 2`, so the gap from virtual line 1 to
/// the first token must exceed 2 for any check to occur. This means 1-2 leading
/// blank lines are never flagged by Layout/EmptyLines (Layout/LeadingEmptyLines
/// handles those). nitrocop was using a flat `consecutive_blanks > max` threshold
/// everywhere, including at the file start.
/// Fix: track whether any non-blank line has been seen; before the first non-blank
/// line, use threshold `max + 1` instead of `max`, matching RuboCop's LINE_OFFSET
/// behavior.
///
/// ## Corpus investigation (2026-03-11)
///
/// FP=1,106 remained. Root cause: RuboCop uses token-based gap detection — it
/// collects line numbers that have tokens, then only checks gaps between
/// consecutive token-bearing lines. Comment-only files (no tokens) get early
/// return with no offenses. Blank lines after the last token line are never
/// checked. nitrocop was checking ALL blank lines (except inside non-code
/// ranges), which produced false positives on blank lines after the last code
/// line (common pattern: trailing comment sections after code).
/// Fix: use the Program node's end offset to find the last code line, and only
/// check blank lines within the code range. Comment-only files (empty Program
/// node where start == end) get early return.
///
/// ## Corpus investigation (2026-03-17)
///
/// FN=228 remained across 37 repos (127 from rubyworks/facets). Root cause:
/// RuboCop's `processed_source.tokens` includes `:tCOMMENT` tokens, so comment
/// lines are token-bearing. The previous fix used only `program_loc.end_offset()`
/// (last AST node line) as the cutoff, which missed blank lines between code
/// and trailing comments. For example, `end\n\n\n# comment` has blank lines
/// between the last code line and the comment line — RuboCop flags them because
/// the comment is a token line, but nitrocop skipped them.
/// Fix: compute `last_token_line` as `max(last_code_line, last_comment_line)`,
/// using `parse_result.comments()` to find comment lines. Comment-only files
/// now also get checked (they have comment tokens). The early return only triggers
/// when there are zero tokens of any kind (no code AND no comments).
pub struct EmptyLines;

impl Cop for EmptyLines {
    fn name(&self) -> &'static str {
        "Layout/EmptyLines"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // RuboCop uses token-based gap detection: it collects line numbers from
        // ALL tokens (including comments), then checks gaps between consecutive
        // token-bearing lines. Files with no tokens at all get early return,
        // and blank lines after the last token line are never checked.
        //
        // In the Parser gem, `processed_source.tokens` includes `:tCOMMENT`
        // tokens for comments. So comment lines ARE token-bearing lines.
        // We replicate this by finding the last token line as the max of the
        // last code line (from the Program node) and the last comment line.
        let program_node = parse_result.node();
        let program_loc = program_node.location();

        let has_code = program_loc.start_offset() != program_loc.end_offset();

        // Find the last code line (1-indexed), or 0 if no code.
        let last_code_line = if has_code {
            let (line, _) = source.offset_to_line_col(program_loc.end_offset().saturating_sub(1));
            line
        } else {
            0
        };

        // Find the last comment line (1-indexed), or 0 if no comments.
        let mut last_comment_line: usize = 0;
        for comment in parse_result.comments() {
            let loc = comment.location();
            let (line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
            if line > last_comment_line {
                last_comment_line = line;
            }
        }

        // The last token line is the max of code and comment lines.
        // If both are 0, there are no tokens at all — early return.
        let last_token_line = last_code_line.max(last_comment_line);
        if last_token_line == 0 {
            return;
        }

        let max = config.get_usize("Max", 1);

        let mut consecutive_blanks = 0;
        let mut byte_offset: usize = 0;
        let lines: Vec<&[u8]> = source.lines().collect();
        let total_lines = lines.len();
        let mut seen_non_blank = false;

        for (i, line) in lines.iter().enumerate() {
            let line_len = line.len() + 1; // +1 for newline
            let current_line = i + 1; // 1-indexed

            if line.is_empty() {
                // Skip the trailing empty element from split() — RuboCop's
                // EmptyLines cop doesn't flag trailing blank lines at EOF
                // (that's Layout/TrailingEmptyLines).
                if i + 1 >= total_lines {
                    break;
                }
                // Skip blank lines after the last token line. RuboCop only
                // checks between consecutive token-bearing lines and never
                // checks past the last token.
                if current_line > last_token_line {
                    byte_offset += line_len;
                    consecutive_blanks = 0;
                    continue;
                }
                // Skip blank lines inside non-code regions (heredocs, strings)
                if !code_map.is_code(byte_offset) {
                    byte_offset += line_len;
                    consecutive_blanks = 0;
                    continue;
                }
                consecutive_blanks += 1;
                // RuboCop starts prev_line=1 with LINE_OFFSET=2, so at the
                // start of a file (before any token), the gap threshold is
                // effectively max+1 instead of max. This means 1-2 leading
                // blank lines are never flagged (Layout/LeadingEmptyLines
                // handles those). Only 3+ leading blanks trigger this cop.
                let threshold = if seen_non_blank { max } else { max + 1 };
                if consecutive_blanks > threshold {
                    let mut diag = self.diagnostic(
                        source,
                        current_line,
                        0,
                        "Extra blank line detected.".to_string(),
                    );
                    if let Some(ref mut corr) = corrections {
                        corr.push(crate::correction::Correction {
                            start: byte_offset,
                            end: byte_offset + line_len,
                            replacement: String::new(),
                            cop_name: self.name(),
                            cop_index: 0,
                        });
                        diag.corrected = true;
                    }
                    diagnostics.push(diag);
                }
            } else {
                consecutive_blanks = 0;
                seen_non_blank = true;
            }
            byte_offset += line_len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{run_cop_full, run_cop_full_with_config};

    crate::cop_fixture_tests!(EmptyLines, "cops/layout/empty_lines");
    crate::cop_autocorrect_fixture_tests!(EmptyLines, "cops/layout/empty_lines");

    #[test]
    fn config_max_2() {
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("Max".into(), serde_yml::Value::Number(2.into()))]),
            ..CopConfig::default()
        };
        // 3 consecutive blank lines should trigger with Max:2
        let source = b"x = 1\n\n\n\ny = 2\n";
        let diags = run_cop_full_with_config(&EmptyLines, source, config.clone());
        assert!(
            !diags.is_empty(),
            "Should fire with Max:2 on 3 consecutive blank lines"
        );

        // 2 consecutive blank lines should NOT trigger with Max:2
        let source2 = b"x = 1\n\n\ny = 2\n";
        let diags2 = run_cop_full_with_config(&EmptyLines, source2, config);
        assert!(
            diags2.is_empty(),
            "Should not fire on 2 consecutive blank lines with Max:2"
        );

        // 2 consecutive blank lines SHOULD trigger with default Max:1
        let diags3 = run_cop_full(&EmptyLines, source2);
        assert!(
            !diags3.is_empty(),
            "Should fire with default Max:1 on 2 consecutive blank lines"
        );
    }

    #[test]
    fn autocorrect_remove_extra_blank() {
        let input = b"x = 1\n\n\ny = 2\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&EmptyLines, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n\ny = 2\n");
    }

    #[test]
    fn autocorrect_remove_multiple_extra() {
        let input = b"x = 1\n\n\n\n\ny = 2\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&EmptyLines, input);
        assert_eq!(corrections.len(), 3); // 4 blanks, max 1, so 3 extra
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n\ny = 2\n");
    }

    #[test]
    fn whitespace_only_lines_are_not_blank() {
        // RuboCop only counts truly empty lines (zero bytes after stripping newline).
        // Lines with only spaces/tabs are NOT blank and should not be counted.
        let source = b"x = 1\n  \n  \ny = 2\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Whitespace-only lines should not be treated as blank: {:?}",
            diags
        );
    }

    #[test]
    fn fire_blanks_in_comment_only_file() {
        // RuboCop's processed_source.tokens includes :tCOMMENT tokens,
        // so comment-only files ARE checked for consecutive blank lines.
        let source = b"# frozen_string_literal: true\n\n\n# Another comment\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            !diags.is_empty(),
            "Should fire on consecutive blank lines in comment-only file"
        );
    }

    #[test]
    fn skip_blanks_between_comment_groups() {
        // Consecutive blank lines between comments ARE checked by RuboCop
        // when there are tokens (code) in the file.
        let source = b"x = 1\n# comment\n\n\n# comment\ny = 2\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            !diags.is_empty(),
            "Should fire on consecutive blank lines between comments when code exists"
        );
    }

    #[test]
    fn fire_blanks_between_code_and_comment() {
        // RuboCop's tokens include comments, so blank lines between
        // code and a trailing comment are checked.
        let source = b"x = 1\n\n\n# trailing comment\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            !diags.is_empty(),
            "Should fire on consecutive blank lines between code and comment"
        );
    }

    #[test]
    fn skip_blanks_after_last_code_no_trailing_comment() {
        // Consecutive blank lines after the last code with no trailing content
        let source = b"x = 1\n\n\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should not fire after last code line: {:?}",
            diags
        );
    }

    #[test]
    fn fire_on_three_blanks_before_first_code() {
        // 3+ blank lines at start: gap from virtual line 1 to first token > LINE_OFFSET(2)
        let source = b"\n\n\nx = 1\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            !diags.is_empty(),
            "Should fire on 3+ blank lines at start of file"
        );
    }

    #[test]
    fn skip_two_blanks_at_start_of_file() {
        // RuboCop starts prev_line=1, so 2 blank lines at start (gap=2)
        // don't exceed LINE_OFFSET=2. Layout/LeadingEmptyLines handles these.
        let source = b"\n\nx = 1\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should not fire on 2 blank lines at start of file: {:?}",
            diags
        );
    }

    #[test]
    fn skip_one_blank_at_start_of_file() {
        // Single blank line at start — never flagged by EmptyLines
        let source = b"\nx = 1\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should not fire on single blank line at start of file: {:?}",
            diags
        );
    }

    #[test]
    fn skip_blanks_in_heredoc() {
        let source = b"x = <<~RUBY\n  foo\n\n\n  bar\nRUBY\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should not fire on blank lines inside heredoc"
        );
    }
}
