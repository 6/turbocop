use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=64, FN=8.
///
/// Fixed sampled FP: the byte-level whitespace check accepted only spaces and
/// line breaks after `,`, but RuboCop also accepts tabs. Corpus examples such
/// as `['ADJ',\t'Adjective']` were therefore false positives. The accepted fix
/// adds `\t` to the allowed post-comma whitespace set and covers that case in
/// the fixture.
///
/// ## Corpus investigation (2026-03-14)
///
/// FP=7 root causes:
/// - 6 FP from line continuation `\` after comma: `,\` at end of line is valid
///   because the continuation merges with the next line. Fixed by skipping commas
///   whose next byte is `\` followed by `\n`.
/// - 1 FP from trailing comma before semicolon in pattern matching (`in 0, 1,;`).
///   Fixed by adding `;` to the allowed-after-comma set (like `)`, `]`, `|`).
///
/// FN=8 root causes:
/// - All FNs are commas inside `#{}` interpolation within heredocs. The CodeMap
///   marks entire heredoc bodies (including interpolation) as non-code, so
///   `is_code()` returns false for these commas. Fixed by also checking
///   `code_map.is_heredoc_interpolation()` which tracks `#{}` content ranges
///   within heredocs separately.
///
/// ## Corpus investigation (2026-03-14, continued)
///
/// FP=200 root cause:
/// - RuboCop's `SpaceAfterPunctuation#space_required_before?` skips commas before
///   `}` when `Layout/SpaceInsideHashLiteralBraces` has `EnforcedStyle: no_space`.
///   Nitrocop was not reading the sibling cop's config. Fixed by injecting
///   `__SpaceInsideHashBracesStyle` from the config layer (same pattern as
///   `MaxLineLength` injection) and skipping comma-before-`}` when `no_space`.
///
/// ## Corpus investigation (2026-03-14, FP=200 remaining)
///
/// All ~200 FPs were commas inside string literals nested within heredoc
/// interpolation. Example: `<<~SQL\n  WHERE id IN (#{ids.join(",")})\nSQL`
/// The comma inside `","` is a string literal within `#{}` inside a heredoc.
/// The `is_heredoc_interpolation()` check correctly identified these offsets as
/// being within heredoc interpolation, but didn't account for nested string
/// literals inside that interpolation. Fixed by adding
/// `heredoc_interpolation_non_code_ranges` to CodeMap which tracks string/regex/
/// symbol literal ranges that are nested within heredoc interpolation blocks,
/// and checking `!is_non_code_in_heredoc_interpolation(i)` in the skip logic.
///
/// ## Corpus investigation (2026-03-16)
///
/// FN=1 root cause:
/// - Comma inside `#{}` interpolation within a string continuation (`"..." \ "..."`).
///   Prism wraps continued strings in an outer `InterpolatedStringNode` with no
///   opening/closing, whose parts include the inner `InterpolatedStringNode` (with
///   `#{}`) as a non-`EmbeddedStatementsNode` part. CodeMap's non-heredoc handler
///   was marking all non-`EmbeddedStatementsNode` parts as non-code, inadvertently
///   covering the inner interpolated string's `#{}` content. Fixed in CodeMap by
///   also skipping `InterpolatedStringNode` parts (the recursive visitor handles
///   them correctly on its own).
///
/// ## Corpus investigation (2026-03-19)
///
/// FP=14 root cause:
/// - All 14 remaining FPs were old-style `%w/%W/%i/%I` array literals using
///   `,` as the delimiter (for example `%i,alpha beta,`). Nitrocop's raw byte
///   scan treated both the opening and closing delimiter commas as punctuation
///   that required a following space, but RuboCop treats percent-literal
///   delimiters as syntax, not comma separators. Fixed by collecting the
///   opening/closing comma offsets for comma-delimited percent arrays from the
///   Prism AST and skipping those offsets during the byte scan.
pub struct SpaceAfterComma;

struct PercentArrayCommaCollector {
    offsets: Vec<usize>,
}

impl<'pr> Visit<'pr> for PercentArrayCommaCollector {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.collect(&node);
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        self.collect(&node);
    }
}

impl PercentArrayCommaCollector {
    fn collect(&mut self, node: &ruby_prism::Node<'_>) {
        let Some(array) = node.as_array_node() else {
            return;
        };
        let Some(open_loc) = array.opening_loc() else {
            return;
        };

        let open = open_loc.as_slice();
        if !open.starts_with(b"%w")
            && !open.starts_with(b"%W")
            && !open.starts_with(b"%i")
            && !open.starts_with(b"%I")
        {
            return;
        }

        if open.last() != Some(&b',') {
            return;
        }

        self.offsets.push(open_loc.end_offset() - 1);

        if let Some(close_loc) = array.closing_loc() {
            if close_loc.as_slice().starts_with(b",") {
                self.offsets.push(close_loc.start_offset());
            }
        }
    }
}

fn comma_delimited_percent_array_offsets(parse_result: &ruby_prism::ParseResult<'_>) -> Vec<usize> {
    let mut collector = PercentArrayCommaCollector {
        offsets: Vec::new(),
    };
    collector.visit(&parse_result.node());
    collector.offsets.sort_unstable();
    collector.offsets.dedup();
    collector.offsets
}

impl Cop for SpaceAfterComma {
    fn name(&self) -> &'static str {
        "Layout/SpaceAfterComma"
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
        let bytes = source.as_bytes();
        let percent_array_delimiter_offsets = comma_delimited_percent_array_offsets(parse_result);
        // RuboCop's SpaceAfterPunctuation#space_required_before? skips commas
        // before `}` when Layout/SpaceInsideHashLiteralBraces uses `no_space`.
        let skip_rcurly = config.get_str("__SpaceInsideHashBracesStyle", "space") == "no_space";
        for (i, &byte) in bytes.iter().enumerate() {
            if byte != b',' {
                continue;
            }
            if percent_array_delimiter_offsets.binary_search(&i).is_ok() {
                continue;
            }
            // Check commas in code regions AND inside heredoc interpolation
            // (but not inside nested string/regex/symbol literals within interpolation)
            if !code_map.is_code(i)
                && (!code_map.is_heredoc_interpolation(i)
                    || code_map.is_non_code_in_heredoc_interpolation(i))
            {
                continue;
            }
            // Skip if this comma is part of a global variable ($, or $;)
            if i > 0 && bytes[i - 1] == b'$' {
                continue;
            }
            let next = bytes.get(i + 1).copied();
            // Skip commas before closing delimiters — RuboCop's
            // SpaceAfterPunctuation#allowed_type? skips ), ], and |.
            // Also skip comma before semicolon (pattern matching: `in 0, 1,;`).
            if matches!(next, Some(b')') | Some(b']') | Some(b'|') | Some(b';')) {
                continue;
            }
            // Skip commas before `}` when SpaceInsideHashLiteralBraces uses no_space
            if next == Some(b'}') && skip_rcurly {
                continue;
            }
            // Skip line continuation: `,\` followed by newline
            if next == Some(b'\\') {
                let after_backslash = bytes.get(i + 2).copied();
                if matches!(after_backslash, Some(b'\n') | Some(b'\r') | None) {
                    continue;
                }
            }
            if !matches!(
                next,
                Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') | None
            ) {
                let (line, column) = source.offset_to_line_col(i);
                let mut diag = self.diagnostic(
                    source,
                    line,
                    column,
                    "Space missing after comma.".to_string(),
                );
                if let Some(ref mut corr) = corrections {
                    corr.push(crate::correction::Correction {
                        start: i + 1,
                        end: i + 1,
                        replacement: " ".to_string(),
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

    crate::cop_fixture_tests!(SpaceAfterComma, "cops/layout/space_after_comma");
    crate::cop_autocorrect_fixture_tests!(SpaceAfterComma, "cops/layout/space_after_comma");

    #[test]
    fn autocorrect_insert_space() {
        let input = b"foo(1,2)\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceAfterComma, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"foo(1, 2)\n");
    }

    #[test]
    fn comma_before_rcurly_no_space_style() {
        // When SpaceInsideHashLiteralBraces uses no_space, comma before } is OK
        let mut options = std::collections::HashMap::new();
        options.insert(
            "__SpaceInsideHashBracesStyle".to_string(),
            serde_yml::Value::String("no_space".to_string()),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &SpaceAfterComma,
            b"{foo: bar,}\n",
            config,
        );
    }

    #[test]
    fn comma_before_rcurly_space_style() {
        // When SpaceInsideHashLiteralBraces uses space (default), comma before } IS an offense
        let mut options = std::collections::HashMap::new();
        options.insert(
            "__SpaceInsideHashBracesStyle".to_string(),
            serde_yml::Value::String("space".to_string()),
        );
        let config = CopConfig {
            options,
            ..CopConfig::default()
        };
        crate::testutil::assert_cop_offenses_full_with_config(
            &SpaceAfterComma,
            b"{foo: bar,}\n         ^ Layout/SpaceAfterComma: Space missing after comma.\n",
            config,
        );
    }

    #[test]
    fn autocorrect_multiple() {
        let input = b"foo(1,2,3)\n";
        let (_diags, corrections) = crate::testutil::run_cop_autocorrect(&SpaceAfterComma, input);
        assert_eq!(corrections.len(), 2);
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"foo(1, 2, 3)\n");
    }
}
