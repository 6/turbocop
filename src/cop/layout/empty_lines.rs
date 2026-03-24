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
///
/// ## Corpus investigation (2026-03-17, FN=21 remaining)
///
/// ~16 FN inside `=begin`/`=end` blocks. Root cause: the cop used
/// `code_map.is_code(byte_offset)` to skip blank lines in non-code regions.
/// The CodeMap marks `=begin`/`=end` block comments as non-code (they are
/// comments), so blank lines inside them were skipped. But RuboCop's
/// `processed_source.tokens` includes `:tEMBDOC` tokens for `=begin`/`=end`
/// content lines, so consecutive blank lines inside them are still flagged.
/// Fix: switched from `is_code()` to `is_not_string()`, which skips
/// strings/heredocs/regexes/symbols but NOT comments (including `=begin`/`=end`).
/// This preserves heredoc/string skipping while allowing `=begin`/`=end`
/// blank line detection.
///
/// ## Corpus investigation (2026-03-17, FP=173 final fix)
///
/// Two root causes for 173 FPs:
///
/// 1. **`=begin`/`=end` blocks** (~125 FPs). At this time, the analysis
///    concluded RuboCop does not flag blank lines inside embdoc blocks.
///    Fix: track `in_embdoc` state during line iteration and skip all
///    interior lines. (This was later found to be incorrect — see
///    2026-03-19 fix below.)
///
/// 2. **CRLF files** (~48 FPs). RuboCop's `EmptyLines` cop has a quick check:
///    `return unless processed_source.raw_source.include?("\n\n\n")`. In CRLF
///    files, blank lines are `\r\n`, so consecutive blanks produce
///    `\r\n\r\n` (no `\n\n\n` substring). RuboCop returns early with 0
///    offenses. Additionally, `processed_source[line].empty?` returns false
///    for `\r` lines, so even without the quick check, CRLF blanks wouldn't
///    fire. Fix: removed `|| *line == [b'\r']` from the `is_blank` check.
///    Only truly empty lines (zero bytes after splitting on `\n`) are blank.
///
/// ## Corpus investigation (2026-03-19, FP=13 + FN=21 fix)
///
/// Two root causes:
///
/// 1. **FP=13: `__END__` included in `last_token_line`**. All 13 FPs were
///    consecutive blank lines between last code and `__END__`. The previous
///    fix included `__END__` (via `data_loc`) in `last_token_line`, but
///    RuboCop 1.84.2 with Prism does NOT include `__END__` as a token in
///    `processed_source.tokens`. Blank lines before `__END__` are past the
///    last token and are never checked.
///    Fix: removed `end_marker_line` from `last_token_line`.
///
/// 2. **FN=20: `=begin` excluded from `last_comment_line`**. Previous code
///    excluded `=begin` block comments from `last_comment_line`. But the
///    `=begin` line extends the token range, so blank lines between code
///    and `=begin` should be checked.
///    Fix: include the `=begin` START line in `last_comment_line`. Track
///    `in_embdoc` state during line iteration to skip lines inside
///    `=begin`/`=end` blocks. Tested including embdoc end lines (to check
///    blanks inside), but this caused 141 new FPs — so embdoc interiors
///    remained skipped.
///
/// ## Corpus investigation (2026-03-19, FN=19 final fix)
///
/// All 19 remaining FN were consecutive blank lines inside `=begin`/`=end`
/// blocks across 6 repos (eventmachine 6, WhatWeb 4, redcar 3, facets 3,
/// treat 2, BubbleWrap 1). Root cause: the previous fix (FP=173) incorrectly
/// concluded that RuboCop does not flag blanks inside embdoc blocks. In fact,
/// RuboCop with Prism generates EMBDOC_BEGIN/EMBDOC_LINE/EMBDOC_END tokens
/// for every line inside `=begin`/`=end`, so all non-blank lines are
/// token-bearing and consecutive blank lines between them ARE flagged.
/// The previous "141 FPs from including embdoc end lines" was caused by
/// extending `last_token_line` to the `=end` line while also skipping
/// embdoc interiors — a conflicting combination. The correct fix:
/// 1. Include the `=end` line in `last_comment_line` so interior blank
///    lines are within the token range.
/// 2. Remove the `in_embdoc` skip entirely — treat embdoc interior lines
///    as normal content for blank line counting.
///
/// Result: FP=0, FN=0. All 19 FN fixed with zero regressions.
///
/// ## Corpus investigation (2026-03-23)
///
/// Corpus oracle reported FP=127, FN=0. Root cause: the 2026-03-19 fix included
/// `=begin`/`=end` block comments in `last_token_line` and treated embdoc interior
/// lines as normal content. This matched Prism's EMBDOC token behavior, but the
/// corpus oracle uses RuboCop with the Parser gem, which does NOT produce tokens
/// for `=begin`/`=end` content. With the Parser gem:
/// - Lines inside `=begin`/`=end` have no tokens, so they're outside the token set
/// - If no code follows `=end`, the embdoc is past the last token and never checked
/// - If code exists both before and after, blank lines inside the block are in the
///   gap between token-bearing lines but are embdoc content, not flagged
///
/// Fix: exclude embdoc block comments from `last_comment_line` computation, and
/// track embdoc ranges to skip blank lines inside `=begin`/`=end` blocks during
/// line iteration. This matches the Parser gem's tokenization behavior.
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
        // ALL tokens (including inline `#` comments via `:tCOMMENT` and
        // EMBDOC_BEGIN/EMBDOC_LINE/EMBDOC_END for `=begin`/`=end` blocks),
        // then checks gaps between consecutive token-bearing lines. Files with
        // no tokens at all get early return, and blank lines after the last
        // token line are never checked.
        //
        // `__END__` is NOT a token in Prism's lexer, so it does not extend
        // the token range. Blank lines before `__END__` are past the last
        // token and are never checked.
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

        // Find the last comment line (1-indexed), or 0 if none.
        // Only count inline (#) comments — RuboCop's Parser gem does NOT
        // produce tokens for =begin/=end block comments, so embdoc blocks
        // don't extend the token range. Blank lines inside embdoc blocks
        // and after the last inline comment are never checked.
        let mut last_comment_line: usize = 0;
        let mut embdoc_ranges: Vec<(usize, usize)> = Vec::new();
        for comment in parse_result.comments() {
            let loc = comment.location();
            let slice = &source.as_bytes()[loc.start_offset()..loc.end_offset()];
            if slice.starts_with(b"=begin") {
                // Track embdoc block range (start line to end line, 1-indexed)
                let (start_line, _) = source.offset_to_line_col(loc.start_offset());
                let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
                embdoc_ranges.push((start_line, end_line));
                continue;
            }
            // Embdoc continuation lines (=end, content lines) — skip
            if slice.starts_with(b"=end") || slice.starts_with(b"=") {
                continue;
            }
            let line = {
                let (l, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
                l
            };
            if line > last_comment_line {
                last_comment_line = line;
            }
        }

        // The last token line is the max of code and comment lines.
        // __END__ is NOT a token in Prism's lexer, so it's excluded.
        // If both are 0, there are no tokens at all — early return.
        let last_token_line = last_code_line.max(last_comment_line);
        if last_token_line == 0 {
            return;
        }

        let max = config.get_usize("Max", 1);

        let mut consecutive_blanks: usize = 0;
        let mut byte_offset: usize = 0;
        let lines: Vec<&[u8]> = source.lines().collect();
        let total_lines = lines.len();
        let mut seen_non_blank = false;
        // Track byte offsets of leading blank lines for deferred emission.
        let mut leading_blank_offsets: Vec<(usize, usize, usize)> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let line_len = line.len() + 1; // +1 for newline
            let current_line = i + 1; // 1-indexed

            // A line is "blank" only if it's truly empty (zero bytes after \n).
            // RuboCop's quick check `raw_source.include?("\n\n\n")` fails for
            // CRLF files (where blank lines are \r\n, not \n), so CRLF blank
            // lines are never flagged. We match this by only treating truly
            // empty lines as blank, NOT \r-only lines.
            let is_blank = line.is_empty();

            if is_blank {
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
                // Skip blank lines inside =begin/=end blocks. The Parser gem
                // doesn't produce tokens for embdoc content, so these are
                // outside the token range from RuboCop's perspective.
                if embdoc_ranges
                    .iter()
                    .any(|&(start, end)| current_line >= start && current_line <= end)
                {
                    byte_offset += line_len;
                    consecutive_blanks = 0;
                    continue;
                }
                // Skip blank lines inside string/heredoc/regex literals.
                // is_not_string() returns false for strings/heredocs/regexes/symbols
                // but true for comments (including =begin/=end) and code.
                if !code_map.is_not_string(byte_offset) {
                    byte_offset += line_len;
                    consecutive_blanks = 0;
                    continue;
                }
                consecutive_blanks += 1;
                if !seen_non_blank {
                    // Defer leading blank line detection. RuboCop uses
                    // prev_line=1 with LINE_OFFSET=2: the gap from line 1
                    // to the first token must exceed 2 (i.e., 3+ leading
                    // blanks) before any check occurs. Then it fires on
                    // each line where both previous and current are empty,
                    // starting at line 2. We collect offsets here and emit
                    // retroactively when the first non-blank line is seen.
                    leading_blank_offsets.push((current_line, byte_offset, line_len));
                } else if consecutive_blanks > max {
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
                // First non-blank line: emit deferred leading blank diagnostics.
                // RuboCop requires gap > LINE_OFFSET(2), meaning 3+ leading
                // blank lines. Then fires on lines 2..N (where both prev and
                // current lines are empty).
                if !seen_non_blank && consecutive_blanks >= max + 2 {
                    // Skip the first blank (line 1): RuboCop's
                    // previous_and_current_lines_empty? needs both prev AND
                    // current empty, so line 1 can't fire (no line 0).
                    // With prev_line=1, the check starts at line 2.
                    for &(ln, off, ll) in &leading_blank_offsets[1..] {
                        let mut diag = self.diagnostic(
                            source,
                            ln,
                            0,
                            "Extra blank line detected.".to_string(),
                        );
                        if let Some(ref mut corr) = corrections {
                            corr.push(crate::correction::Correction {
                                start: off,
                                end: off + ll,
                                replacement: String::new(),
                                cop_name: self.name(),
                                cop_index: 0,
                            });
                            diag.corrected = true;
                        }
                        diagnostics.push(diag);
                    }
                }
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
        // Should fire on lines 2 and 3 (2 offenses, not 1).
        let source = b"\n\n\nx = 1\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert_eq!(
            diags.len(),
            2,
            "Should fire twice on 3 blank lines at start of file: {:?}",
            diags
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

    #[test]
    fn skip_blanks_inside_begin_end_with_code_after() {
        // RuboCop's Parser gem does NOT produce tokens for =begin/=end content,
        // so blank lines inside embdoc blocks are never checked.
        let source = b"=begin\nsome docs\n\n\nmore docs\n=end\nx = 1\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should NOT fire on blank lines inside =begin/=end: {:?}",
            diags
        );
    }

    #[test]
    fn skip_many_blanks_inside_begin_end_with_code_after() {
        // Multiple consecutive blank lines inside =begin/=end are NOT flagged.
        let source = b"=begin\n\n\n\n\n=end\nx = 1\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should NOT fire on blank lines inside =begin/=end: {:?}",
            diags
        );
    }

    #[test]
    fn skip_blanks_in_begin_end_no_code_after() {
        // Blank lines inside =begin/=end are NOT flagged even with code before.
        let source = b"x = 1\n=begin\n\n\n\n\n=end\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should NOT fire on blank lines inside =begin/=end: {:?}",
            diags
        );
    }

    #[test]
    fn fire_blanks_outside_begin_end_block() {
        // Blank lines OUTSIDE =begin/=end should still be flagged.
        let source = b"=begin\ndocs\n=end\n\n\nx = 1\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            !diags.is_empty(),
            "Should fire on consecutive blank lines outside =begin/=end"
        );
    }

    #[test]
    fn skip_blanks_before_begin_end() {
        // With the Parser gem, =begin is not a token. Blank lines between
        // code and =begin are after the last token and are not checked.
        let source = b"x = 1\n\n\n=begin\nsome docs\n=end\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should NOT fire on blank lines before =begin (past last token): {:?}",
            diags
        );
    }

    #[test]
    fn skip_many_blanks_before_begin_end() {
        // Multiple blanks before =begin, no code after =end — all past last token.
        let source = b"x = 1\n\n\n\n=begin\nmore docs\n=end\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should NOT fire on blank lines before =begin (past last token): {:?}",
            diags
        );
    }

    #[test]
    fn skip_blanks_crlf_line_endings() {
        // RuboCop's quick check `raw_source.include?("\n\n\n")` fails for
        // CRLF files because blank lines are `\r\n\r\n` (no triple \n).
        // So RuboCop never fires on CRLF files. We match this behavior by
        // not treating `\r`-only lines as blank.
        let source = b"x = 1\r\n\r\n\r\ny = 2\r\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should NOT fire on consecutive blank CRLF lines: {:?}",
            diags
        );
    }

    #[test]
    fn skip_blanks_before_end_marker() {
        // __END__ is NOT a token in Prism's lexer. Blank lines before it
        // are past the last token line and are never checked.
        let source = b"x = 1\n\n\n__END__\ndata here\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should NOT fire on blank lines before __END__: {:?}",
            diags
        );
    }

    #[test]
    fn skip_blanks_after_end_marker() {
        // Blank lines inside the __END__ data section should NOT be checked.
        let source = b"x = 1\n__END__\n\n\ndata\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should not fire on blank lines inside __END__ data: {:?}",
            diags
        );
    }

    #[test]
    fn skip_blanks_crlf_single_blank_is_fine() {
        // Single blank line in CRLF should not fire either.
        let source = b"x = 1\r\n\r\ny = 2\r\n";
        let diags = run_cop_full(&EmptyLines, source);
        assert!(
            diags.is_empty(),
            "Should not fire on single blank CRLF line: {:?}",
            diags
        );
    }
}
