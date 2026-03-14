use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

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
pub struct SpaceAfterComma;

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
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let bytes = source.as_bytes();
        // RuboCop's SpaceAfterPunctuation#space_required_before? skips commas
        // before `}` when Layout/SpaceInsideHashLiteralBraces uses `no_space`.
        let skip_rcurly = config.get_str("__SpaceInsideHashBracesStyle", "space") == "no_space";
        for (i, &byte) in bytes.iter().enumerate() {
            if byte != b',' {
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
