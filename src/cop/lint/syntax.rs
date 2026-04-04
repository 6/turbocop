use crate::cop::Cop;
use crate::diagnostic::Severity;

const INVALID_RETRY_WITHOUT_RESCUE: &str = "Invalid retry without rescue";
const INVALID_RETURN_IN_CLASS_MODULE: &str = "Invalid return in class/module body";

/// Returns true if the parse error is one that RuboCop also reports as Lint/Syntax
/// even though Prism classifies it as a "semantic" parse error.
pub fn is_rubocop_reported_semantic_error(message: &str) -> bool {
    message == INVALID_RETRY_WITHOUT_RESCUE || message.starts_with(INVALID_RETURN_IN_CLASS_MODULE)
}

/// Checks for syntax errors.
///
/// This cop is a registration stub — the actual detection logic lives in
/// `emit_syntax_diagnostics()` in `src/linter.rs`. When a file has structural
/// parse errors (detected by Prism), each error is emitted as a Lint/Syntax
/// offense with Fatal severity, matching RuboCop's behavior of repacking
/// parser diagnostics into Lint/Syntax offenses.
///
/// ## Corpus investigation (2026-03-24)
///
/// FN=4708: nitrocop silently skipped files with parse errors (returning empty
/// diagnostics). RuboCop's Lint/Syntax reports each parser error/fatal diagnostic
/// as a separate offense. Fixed by adding `emit_syntax_diagnostics()` to the
/// linter pipeline that emits one Lint/Syntax diagnostic per structural Prism
/// error when the cop is enabled.
///
/// ## Corpus investigation (2026-03-29)
///
/// FN=183, FP=21: off-by-one line numbers for parse errors at end-of-file.
/// Prism reports "end-of-input" (and other EOF) errors at offset == file_size,
/// which it considers line N+1 for an N-line file ending with `\n`. Our
/// `offset_to_line_col()` mapped that offset to line N instead. Fixed by
/// detecting the at-or-past-end case in `emit_syntax_diagnostics()` and
/// incrementing the line number to match Prism/RuboCop. This resolved 162 FN
/// and all 21 FP (which were the same errors reported at the wrong line).
///
/// ## Corpus investigation (2026-03-30)
///
/// FN=27: files with invalid UTF-8 bytes (and no encoding magic comment) were
/// silently skipped with empty diagnostics. RuboCop reports these as a fatal
/// Lint/Syntax "Invalid byte sequence in utf-8." offense at line 1. Fixed by
/// adding `emit_invalid_utf8_diagnostic()` in `lint_file()` to emit the
/// diagnostic instead of returning empty. Resolved 21 of 27 FN.
///
/// FN=2: standalone `retry` statements were still missed because the linter's
/// semantic-error filter suppresses Prism's `Invalid retry without rescue`
/// parse error to avoid broader semantic-error false positives. Fixed by
/// re-emitting only that exact parse error from this cop's `check_source`
/// hook, which runs on files without structural parse failures. Other semantic
/// Prism errors remain filtered in the linter because broad emission caused
/// false positives in corpus validation.
///
/// ## Corpus investigation (2026-04-04)
///
/// FN=4: two semantic parse errors that RuboCop reports as Lint/Syntax were
/// being suppressed in files with structural errors. `emit_syntax_diagnostics`
/// in `src/linter.rs` filtered ALL semantic errors, but RuboCop still emits
/// "Invalid retry without rescue" and "Invalid return in class/module body"
/// even alongside structural errors. Fixed by adding
/// `is_rubocop_reported_semantic_error()` to exempt these two error types
/// from the semantic filter in `emit_syntax_diagnostics`. Also added
/// "Invalid return in class/module body" to `check_source` for files
/// without structural errors. Resolved all 4 FN.
pub struct Syntax;

impl Cop for Syntax {
    fn name(&self) -> &'static str {
        "Lint/Syntax"
    }

    fn default_severity(&self) -> Severity {
        Severity::Fatal
    }

    fn check_source(
        &self,
        source: &crate::parse::source::SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &crate::cop::CopConfig,
        diagnostics: &mut Vec<crate::diagnostic::Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        for err in parse_result.errors() {
            if !is_rubocop_reported_semantic_error(err.message()) {
                continue;
            }

            let (line, column) = source.offset_to_line_col(err.location().start_offset());
            diagnostics.push(self.diagnostic(source, line, column, err.message().to_string()));
        }
    }

    // Syntax errors are reported by the parser (Prism), not by this cop.
    // This struct also handles the narrow bare-`retry` semantic parse error
    // that Prism reports but the linter-wide structural-error path suppresses.
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Syntax, "cops/lint/syntax");

    #[test]
    fn cop_name() {
        assert_eq!(Syntax.name(), "Lint/Syntax");
    }

    #[test]
    fn default_severity_is_fatal() {
        assert_eq!(Syntax.default_severity(), Severity::Fatal);
    }

    #[test]
    fn no_offenses_on_valid_source() {
        use crate::testutil::run_cop_full;
        let source = b"x = 1\ny = 2\n";
        let diags = run_cop_full(&Syntax, source);
        assert!(diags.is_empty());
    }

    /// Helper: build standard test args with --only Lint/Syntax.
    fn syntax_only_args() -> crate::cli::Args {
        crate::cli::Args {
            paths: vec![],
            config: None,
            format: "text".to_string(),
            only: vec!["Lint/Syntax".to_string()],
            except: vec![],
            no_color: false,
            debug: false,
            rubocop_only: false,
            list_cops: false,
            list_autocorrectable_cops: false,
            migrate: false,
            doctor: false,
            rules: false,
            tier: None,
            stdin: None,
            init: false,
            no_cache: false,
            cache: "true".to_string(),
            cache_clear: false,
            fail_level: "convention".to_string(),
            fail_fast: false,
            force_exclusion: false,
            list_target_files: false,
            display_cop_names: false,
            parallel: false,
            require_libs: vec![],
            ignore_disable_comments: false,
            force_default_config: false,
            autocorrect: false,
            autocorrect_all: false,
            preview: true,
            quiet_skips: false,
            strict: None,
            verify: false,
            rubocop_cmd: "bundle exec rubocop".to_string(),
            corpus_check: None,
        }
    }

    /// Helper: lint raw bytes through the full pipeline (including syntax diagnostics).
    fn lint_bytes(source_bytes: &[u8]) -> Vec<crate::diagnostic::Diagnostic> {
        use crate::config::ResolvedConfig;
        use crate::cop::registry::CopRegistry;
        use crate::cop::tiers::TierMap;
        use crate::parse::source::SourceFile;

        let source = SourceFile::from_bytes("test.rb", source_bytes.to_vec());
        let registry = CopRegistry::default_registry();
        let tier_map = TierMap::load();
        let config = ResolvedConfig::empty();
        let cop_filters = config.build_cop_filters(&registry, &tier_map, true);
        let base_configs = config.precompute_cop_configs(&registry);
        let args = syntax_only_args();
        let allowlist = crate::cop::autocorrect_allowlist::AutocorrectAllowlist::load();

        let (diags, _, _) = crate::linter::lint_source_inner(
            &source,
            &config,
            &registry,
            &args,
            &tier_map,
            &cop_filters,
            &base_configs,
            false,
            None,
            &allowlist,
        );
        diags
    }

    /// Test that syntax errors at end-of-file get the correct line number.
    /// Prism reports "end-of-input" errors at offset == file_size, which for
    /// files ending with \n is one line past the last content line. The linter
    /// must match Prism's (and RuboCop's) line numbering.
    #[test]
    fn end_of_input_line_number_matches_prism() {
        // An ERB template fragment that causes "unexpected end-of-input" at EOF.
        // 3 content lines, ends with \n. Prism will report the end-of-input
        // error at offset == file_size, which it considers line 4.
        let diags = lint_bytes(b"class <%= name %>\nend\nend\n");

        let eoi_diag = diags.iter().find(|d| d.message.contains("end-of-input"));
        assert!(
            eoi_diag.is_some(),
            "Expected an end-of-input diagnostic, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        let eoi = eoi_diag.unwrap();
        // File has 3 content lines; end-of-input should be on line 4
        assert_eq!(
            eoi.location.line, 4,
            "end-of-input should be on line 4 (one past last content line), got line {}",
            eoi.location.line
        );
    }

    /// Test that invalid UTF-8 bytes trigger "Invalid byte sequence in utf-8."
    /// RuboCop reports this as a global Lint/Syntax offense at line 1.
    #[test]
    fn invalid_utf8_is_reported() {
        use crate::config::ResolvedConfig;
        use crate::cop::registry::CopRegistry;
        use crate::cop::tiers::TierMap;
        use crate::parse::source::SourceFile;

        // File with invalid UTF-8 byte 0xc0 0x80 (overlong encoding)
        let source_bytes: &[u8] = b"# \xc0\x80 test\n";
        let source = SourceFile::from_bytes("test.rb", source_bytes.to_vec());
        let registry = CopRegistry::default_registry();
        let tier_map = TierMap::load();
        let config = ResolvedConfig::empty();
        let cop_filters = config.build_cop_filters(&registry, &tier_map, true);
        let args = syntax_only_args();

        // Use emit_invalid_utf8_diagnostic through lint_file indirectly.
        // Since lint_source_inner doesn't check UTF-8 (that's in lint_file),
        // we test emit_invalid_utf8_diagnostic directly via the linter module.
        let diags = crate::linter::emit_invalid_utf8_diagnostic(
            &source,
            &config,
            &registry,
            &cop_filters,
            false,
            &tier_map,
            &args,
        );

        assert_eq!(diags.len(), 1, "Expected 1 diagnostic, got {:?}", diags);
        let d = &diags[0];
        assert_eq!(d.cop_name, "Lint/Syntax");
        assert_eq!(d.message, "Invalid byte sequence in utf-8.");
        assert_eq!(d.location.line, 1);
        assert_eq!(d.location.column, 0);
    }

    #[test]
    fn invalid_retry_without_rescue_is_reported() {
        let diags = lint_bytes(b"retry\n");

        assert_eq!(diags.len(), 1, "Expected 1 diagnostic, got {:?}", diags);
        let d = &diags[0];
        assert_eq!(d.cop_name, "Lint/Syntax");
        assert_eq!(d.message, INVALID_RETRY_WITHOUT_RESCUE);
        assert_eq!(d.location.line, 1);
        assert_eq!(d.location.column, 0);
    }

    #[test]
    fn retry_inside_rescue_is_not_reported() {
        let diags = lint_bytes(b"begin\nrescue StandardError\n  retry\nend\n");
        assert!(
            diags.is_empty(),
            "Expected no diagnostics for retry inside rescue, got {:?}",
            diags
        );
    }

    /// Test that "Invalid retry without rescue" is reported even in files that
    /// also have structural parse errors (e.g., `%` causing unterminated string).
    /// Regression test for FN in ruby-syntax-tree/syntax_tree retry.rb.
    #[test]
    fn retry_reported_alongside_structural_errors() {
        // `%` on its own is a structural parse error; lines 4 and 9 have `retry`
        // inside begin/rescue/end but Prism still flags them as invalid retry
        // because error recovery after `%` doesn't see the rescue context.
        let source = b"%\nbegin\nrescue StandardError\n  retry\nend\n%\nbegin\nrescue StandardError\n  retry\nend\n";
        let diags = lint_bytes(source);
        let retry_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message == INVALID_RETRY_WITHOUT_RESCUE)
            .collect();
        assert!(
            !retry_diags.is_empty(),
            "Expected 'Invalid retry without rescue' in file with structural errors, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// Test that "Invalid return in class/module body" is reported.
    #[test]
    fn invalid_return_in_class_module_body_is_reported() {
        let diags = lint_bytes(b"class Foo\n  return 1\nend\n");
        let return_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.starts_with(INVALID_RETURN_IN_CLASS_MODULE))
            .collect();
        assert_eq!(
            return_diags.len(),
            1,
            "Expected 1 'Invalid return in class/module body' diagnostic, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert_eq!(return_diags[0].cop_name, "Lint/Syntax");
    }
}
