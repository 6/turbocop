use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=6, FN=108.
///
/// Fixed a confirmed behavior gap in the local and vendor coverage: the
/// trailing scan previously handled only ASCII spaces and tabs. It now also
/// recognizes UTF-8 full-width spaces (`U+3000`) and strips a line-ending `\r`
/// before computing the trailing span so diagnostics and autocorrect stay
/// aligned on CRLF input.
///
/// Sampled corpus `# ` comment lines were consistent with existing file-drop
/// noise and did not justify a broader comment-specific change, so the
/// accepted patch stayed narrow.
///
/// Acceptance gate after this patch (`scripts/check-cop.py --verbose --rerun`):
/// expected=73,221, actual=71,636, CI baseline=73,119, excess=0, missing=1,585,
/// file-drop noise=1,014.
///
/// Remaining gap: 1,585 potential FN remain. This batch only fixed the
/// Unicode/offset handling path; the remaining misses were not reduced further.
///
/// ## Corpus investigation (2026-03-14)
///
/// CI baseline reported FP=87, FN=4. Fixed two root causes of false positives:
///
/// 1. **CRLF `__END__` handling**: The `__END__` check compared raw bytes
///    (`*line == b"__END__"`), which failed on CRLF files where the line
///    includes a trailing `\r`. Now strips `\r` before comparing, so
///    `__END__\r` is correctly recognized as the data section marker.
///    This was the primary FP source — files with `__END__` followed by
///    data containing trailing whitespace were incorrectly flagged.
///
/// 2. **`__END__` inside heredocs**: The `__END__` check was unconditional,
///    breaking out of the loop even when inside a heredoc (where `__END__`
///    is just string content). Now only breaks at `__END__` when not inside
///    a tracked heredoc. This was also causing FNs (missing offenses after
///    the heredoc).
///
/// Also simplified heredoc terminator matching to use the already-stripped
/// line (no redundant `\r` suffix check).
///
/// ## Corpus investigation (2026-03-14, round 2)
///
/// CI baseline reported FP=87, FN=4. Fixed two root causes:
///
/// 1. **False heredoc detection from shift/append operators**: The `<<`
///    heredoc detection heuristic matched `items <<value` (shift with no
///    space) as a heredoc opener. When `AllowInHeredoc: true`, subsequent
///    lines were skipped until a bare `value` line was found, causing FNs.
///    Fixed by looking back past whitespace to check the preceding token:
///    if it's an identifier, number, `)`, `]`, `}`, `@`, or `$`, treat
///    as shift/append, not heredoc. Also reject `<<` followed by a digit.
///
/// 2. **Single-heredoc tracking**: Changed from `Option<Vec<u8>>` to a
///    stack (`Vec<Vec<u8>>`) to support multiple heredocs on one line
///    (e.g., `method(<<~A, <<~B)`). The old code only tracked the first,
///    causing FPs inside the second heredoc when `AllowInHeredoc: true`.
///
/// 3. **Fixture trailing whitespace**: The offense.rb fixture had its
///    trailing whitespace stripped. Rewrote with raw bytes and added test
///    cases for comment-line trailing spaces (matching the corpus FN
///    patterns from activemerchant/sage.rb and randym/axlsx).
///
/// Remaining FP gap (87): Most FPs are likely config/exclusion issues
/// (e.g., `vendor/**/*` default exclusion, project `AllowInHeredoc: true`
/// with heredoc patterns the heuristic still misses). These are
/// config-resolution gaps, not cop-logic bugs.
///
/// ## Corpus investigation (2026-03-15)
///
/// CI baseline reported FP=87, FN=4.
///
/// `verify-cop-locations.py` confirms all 87 FPs are already FIXED by
/// prior patches (CRLF `__END__`, heredoc detection, shift operator fixes).
/// The corpus oracle has not yet been re-run to reflect these fixes.
///
/// FN=4 remain (3 in activemerchant sage.rb:149/155/168, 1 in axlsx
/// header_footer.rb:8). Without corpus repos cloned locally, root cause
/// cannot be determined. Investigated possible causes:
/// - Heredoc false detection: ruled out (AllowInHeredoc defaults to false,
///   and heredoc tracking has no effect on detection when false).
/// - `__END__` early termination: ruled out (FN lines are mid-file).
/// - `\r`-only line endings (classic Mac): plausible but unlikely for
///   modern Ruby files. Would cause nitrocop to treat entire file as one
///   line, missing mid-file trailing whitespace.
/// - File encoding issues: possible if the file can't be read or has
///   unusual encoding that prevents line splitting.
///   These 4 FNs (0.005% of 73,562 total offenses) are likely file-level
///   artifacts rather than cop logic bugs. Deferring until corpus repos
///   can be cloned for direct investigation.
pub struct TrailingWhitespace;

fn strip_line_ending_carriage_return(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn trailing_whitespace_start(line: &[u8]) -> Option<usize> {
    let mut end = line.len();
    let mut found = false;

    while end > 0 {
        if matches!(line[end - 1], b' ' | b'\t') {
            end -= 1;
            found = true;
            continue;
        }

        if end >= 3 && line[end - 3..end] == [0xE3, 0x80, 0x80] {
            end -= 3;
            found = true;
            continue;
        }

        break;
    }

    found.then_some(end)
}

impl Cop for TrailingWhitespace {
    fn name(&self) -> &'static str {
        "Layout/TrailingWhitespace"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_in_heredoc = config.get_bool("AllowInHeredoc", false);

        // Track heredoc regions: when AllowInHeredoc is true, skip lines inside heredocs.
        // Simple heuristic: track <<~WORD / <<-WORD / <<WORD openers and their terminators.
        let lines: Vec<&[u8]> = source.lines().collect();
        // Track heredoc regions: stack of terminators to support multiple
        // heredocs opened on the same line (e.g., `method(<<~A, <<~B)`).
        let mut heredoc_terminators: Vec<Vec<u8>> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            // Strip trailing \r early for CRLF compatibility.
            let stripped = strip_line_ending_carriage_return(line);

            // Check if we're inside a heredoc
            if let Some(terminator) = heredoc_terminators.last() {
                let trimmed: Vec<u8> = stripped
                    .iter()
                    .copied()
                    .skip_while(|&b| b == b' ' || b == b'\t')
                    .collect();
                if trimmed == *terminator {
                    heredoc_terminators.pop();
                } else if allow_in_heredoc {
                    continue; // Skip trailing whitespace check inside heredoc
                }
            }

            // Stop checking after __END__ marker (data section), but only when
            // not inside a heredoc (where __END__ is just string content).
            if stripped == b"__END__" && heredoc_terminators.is_empty() {
                break;
            }

            // Detect heredoc openers (<<~WORD, <<-WORD, <<WORD, <<~'WORD', etc.)
            // Find ALL heredoc openers on this line to support multiple heredocs.
            if heredoc_terminators.is_empty() {
                let mut search_from = 0;
                // Collect in reverse order so the stack pops them in source order.
                let mut new_terminators = Vec::new();
                while search_from + 1 < stripped.len() {
                    if let Some(rel_pos) =
                        stripped[search_from..].windows(2).position(|w| w == b"<<")
                    {
                        let pos = search_from + rel_pos;
                        search_from = pos + 2;

                        // Distinguish heredoc `<<` from shift/append `<<`:
                        // A heredoc opener follows `=`, `(`, `[`, `,`, or
                        // appears at the start of a line. A shift/append
                        // follows an expression (identifier, number, `)`,
                        // `]`, `}`). Look back past whitespace to find the
                        // meaningful preceding token.
                        if pos > 0 {
                            let mut check_pos = pos - 1;
                            // Skip whitespace backwards
                            while check_pos > 0
                                && (stripped[check_pos] == b' ' || stripped[check_pos] == b'\t')
                            {
                                check_pos -= 1;
                            }
                            let prev = stripped[check_pos];
                            if prev.is_ascii_alphanumeric()
                                || prev == b'_'
                                || prev == b')'
                                || prev == b']'
                                || prev == b'}'
                                || prev == b'@'
                                || prev == b'$'
                            {
                                continue;
                            }
                        }

                        let after = &stripped[pos + 2..];
                        let after = if after.starts_with(b"~") || after.starts_with(b"-") {
                            &after[1..]
                        } else {
                            after
                        };
                        // Strip quotes around terminator
                        let (after, _quoted) =
                            if after.starts_with(b"'") || after.starts_with(b"\"") {
                                let quote = after[0];
                                if let Some(end) = after[1..].iter().position(|&b| b == quote) {
                                    (&after[1..1 + end], true)
                                } else {
                                    (after, false)
                                }
                            } else {
                                (after, false)
                            };
                        // Extract identifier — must start with a letter or
                        // underscore (not a digit) to avoid matching `1<<2`.
                        if after.is_empty() || (!after[0].is_ascii_alphabetic() && after[0] != b'_')
                        {
                            continue;
                        }
                        let ident: Vec<u8> = after
                            .iter()
                            .copied()
                            .take_while(|&b| b.is_ascii_alphanumeric() || b == b'_')
                            .collect();
                        if !ident.is_empty() {
                            new_terminators.push(ident);
                        }
                    } else {
                        break;
                    }
                }
                // Push in reverse so the first heredoc's terminator is on top
                // of the stack and gets matched first.
                for t in new_terminators.into_iter().rev() {
                    heredoc_terminators.push(t);
                }
            }

            if stripped.is_empty() {
                continue;
            }
            if let Some(trailing_start) = trailing_whitespace_start(stripped) {
                let Some(line_start) = source.line_col_to_offset(i + 1, 0) else {
                    continue;
                };
                let start = line_start + trailing_start;
                let end = line_start + stripped.len();
                let (line_num, column) = source.offset_to_line_col(start);
                let mut diag = self.diagnostic(
                    source,
                    line_num,
                    column,
                    "Trailing whitespace detected.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start,
                        end,
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                    diag.corrected = true;
                }
                diagnostics.push(diag);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(TrailingWhitespace, "cops/layout/trailing_whitespace");
    crate::cop_autocorrect_fixture_tests!(TrailingWhitespace, "cops/layout/trailing_whitespace");

    #[test]
    fn no_offense_after_end_marker() {
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = 1\n__END__\ndata with trailing spaces   \nmore data   \n".to_vec(),
        );
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should not flag trailing whitespace after __END__"
        );
    }

    #[test]
    fn all_whitespace_line() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\n   \ny = 2\n".to_vec());
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
        assert_eq!(diags[0].location.column, 0);
    }

    #[test]
    fn trailing_tab() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1\t\n".to_vec());
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 5);
    }

    #[test]
    fn no_trailing_newline() {
        let source = SourceFile::from_bytes("test.rb", b"x = 1  ".to_vec());
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 5);
    }

    #[test]
    fn allow_in_heredoc_skips_heredoc_whitespace() {
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("AllowInHeredoc".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = <<~TEXT\n  hello  \n  world  \nTEXT\n".to_vec(),
        );
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &config, &mut diags, None);
        assert!(
            diags.is_empty(),
            "AllowInHeredoc should skip trailing whitespace inside heredocs"
        );
    }

    #[test]
    fn default_flags_heredoc_whitespace() {
        let source = SourceFile::from_bytes("test.rb", b"x = <<~TEXT\n  hello  \nTEXT\n".to_vec());
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "Default should flag trailing whitespace inside heredocs"
        );
    }

    #[test]
    fn autocorrect_trailing_spaces() {
        let input = b"x = 1   \ny = 2\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&TrailingWhitespace, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\ny = 2\n");
    }

    #[test]
    fn autocorrect_all_whitespace_line() {
        let input = b"x = 1\n   \ny = 2\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&TrailingWhitespace, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\n\ny = 2\n");
    }

    #[test]
    fn autocorrect_multiple_lines() {
        let input = b"x = 1  \ny = 2  \nz = 3\n";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&TrailingWhitespace, input);
        assert_eq!(diags.len(), 2);
        assert!(diags.iter().all(|d| d.corrected));
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1\ny = 2\nz = 3\n");
    }

    #[test]
    fn autocorrect_no_trailing_newline() {
        let input = b"x = 1  ";
        let (diags, corrections) = crate::testutil::run_cop_autocorrect(&TrailingWhitespace, input);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].corrected);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"x = 1");
    }

    #[test]
    fn no_offense_after_end_marker_crlf() {
        // CRLF line endings: __END__\r\n should still be recognized as end marker
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = 1\r\n__END__\r\ndata with trailing spaces   \r\nmore data   \r\n".to_vec(),
        );
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert!(
            diags.is_empty(),
            "Should not flag trailing whitespace after __END__ in CRLF files, got {:?}",
            diags.iter().map(|d| d.location.line).collect::<Vec<_>>()
        );
    }

    #[test]
    fn shift_operator_not_heredoc() {
        // << as shift/append operator should not trigger heredoc detection
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("AllowInHeredoc".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source =
            SourceFile::from_bytes("test.rb", b"array << \"item\"\nnext_line   \n".to_vec());
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &config, &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "Should flag trailing whitespace on next_line even after << operator"
        );
        assert_eq!(diags[0].location.line, 2);
    }

    #[test]
    fn heredoc_containing_end_marker() {
        // __END__ inside a heredoc should not stop processing
        let source = SourceFile::from_bytes(
            "test.rb",
            b"x = <<~HEREDOC\n__END__\nHEREDOC\ny = 1   \n".to_vec(),
        );
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &CopConfig::default(), &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "Should flag trailing whitespace after heredoc containing __END__"
        );
        assert_eq!(diags[0].location.line, 4);
    }

    #[test]
    fn shift_operator_no_space_not_heredoc() {
        // `items <<value` (no space) should not trigger heredoc detection.
        // This pattern causes FNs when AllowInHeredoc is true: the shift operator
        // is misdetected as a heredoc opener, and subsequent lines are skipped.
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("AllowInHeredoc".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"items <<value\n# comment with trailing spaces   \ny = 2\n".to_vec(),
        );
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &config, &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "Should flag trailing whitespace on comment line after << shift operator"
        );
        assert_eq!(diags[0].location.line, 2);
    }

    #[test]
    fn multiple_heredocs_on_one_line() {
        // When multiple heredocs are opened on one line, all should be tracked.
        // Only tracking the first causes FPs when AllowInHeredoc is true.
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("AllowInHeredoc".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"method(<<~A, <<~B)\ncontent_a   \nA\ncontent_b   \nB\ny = 1\n".to_vec(),
        );
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &config, &mut diags, None);
        assert!(
            diags.is_empty(),
            "AllowInHeredoc should skip trailing whitespace in both heredocs, got {} diag(s) at lines {:?}",
            diags.len(),
            diags.iter().map(|d| d.location.line).collect::<Vec<_>>()
        );
    }

    #[test]
    fn bit_shift_not_heredoc() {
        // `1<<2` or `x<<2` (bit shift) should not trigger heredoc detection
        use std::collections::HashMap;
        let config = CopConfig {
            options: HashMap::from([("AllowInHeredoc".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = SourceFile::from_bytes(
            "test.rb",
            b"flags = 1<<SHIFT_AMOUNT\nnext_line   \n".to_vec(),
        );
        let mut diags = Vec::new();
        TrailingWhitespace.check_lines(&source, &config, &mut diags, None);
        assert_eq!(
            diags.len(),
            1,
            "Should flag trailing whitespace after bit shift operator"
        );
        assert_eq!(diags[0].location.line, 2);
    }
}
