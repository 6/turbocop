use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=19, FN=199.
///
/// The dominant FN family was compact multi-hash comments like `##patterns`
/// and `##$FUNCTOR_EXCEPTIONS`, especially in `facets`, `axlsx`, `chatwoot`,
/// and `rufo`. RuboCop only accepts multiple leading `#` characters when the
/// run is followed by whitespace or the comment ends; the old matcher skipped
/// every comment starting with `##`, which suppressed those offenses.
///
/// This pass narrows that exemption so `## section header` and `######` remain
/// accepted, while `##foo` is flagged like RuboCop. Remaining FP/FN, if any,
/// are likely in the config-gated comment families (`#ruby`, RBS inline,
/// Steep annotations, shebang continuation) rather than the compact `##...`
/// shape fixed here.
///
/// ## Corpus investigation (2026-03-14)
///
/// FP=5, FN=10.
///
/// FP root causes:
/// - 3 FPs: `#\ -p 4000` on line 1 of `config.ru` files. RuboCop allows
///   `#\` as rackup options on the first line of `config.ru`. Fixed by
///   checking filename + line 1 + `#\` prefix.
/// - 2 FPs: `#~# ORIGINAL` in `.rb.spec` files (rufo). These are
///   file-discovery differences — RuboCop doesn't process `.rb.spec`
///   files at all. Not a cop logic issue; no cop change needed.
///
/// FN root cause: All 10 FNs were `#!` comments NOT on line 1 (e.g.,
/// `#!self.collection_items.unrevealed.empty?`). The old code skipped
/// ALL `#!` comments as shebangs, but RuboCop only allows `#!` on the
/// very first line of the file. Fixed by checking line number.
///
/// ## Corpus investigation (2026-03-15)
///
/// CI reports FP=2, FN=0 on `#~# ORIGINAL` / `#~# EXPECTED` lines in rufo's
/// formatter fixtures.
///
/// Local reduction reproduced the exact two false positives with this
/// six-line file:
///
/// ```text
/// #~# ORIGINAL retry
///
/// retry
///
/// #~# EXPECTED
/// retry
/// ```
///
/// Root cause: this is a syntax-error file (`retry` outside `rescue`), not a
/// special `#~#` comment shape. RuboCop's commissioner only runs
/// `on_new_investigation` cops when `processed_source.valid_syntax?` is true,
/// so `Layout/LeadingCommentSpace` is skipped entirely. nitrocop's shared
/// linter still allows some Prism parse errors through, so this cop was seeing
/// the comments and flagging them.
///
/// Fix: bail out of `check_source` when Prism reported parse errors, matching
/// RuboCop's valid-syntax gate for this cop. A broader shared parse-error
/// policy cleanup may still be needed separately.
pub struct LeadingCommentSpace;

impl Cop for LeadingCommentSpace {
    fn name(&self) -> &'static str {
        "Layout/LeadingCommentSpace"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _allow_doxygen = config.get_bool("AllowDoxygenCommentStyle", false);
        let _allow_gemfile_ruby = config.get_bool("AllowGemfileRubyComment", false);
        let _allow_rbs_inline = config.get_bool("AllowRBSInlineAnnotation", false);
        let _allow_steep = config.get_bool("AllowSteepAnnotation", false);
        let bytes = source.as_bytes();

        if parse_result.errors().next().is_some() {
            return;
        }

        for comment in parse_result.comments() {
            let loc = comment.location();
            let start = loc.start_offset();
            let end = loc.end_offset();
            let text = &bytes[start..end];

            if !missing_space_after_hash(text) {
                continue;
            }

            let (line, column) = source.offset_to_line_col(start);

            // Skip shebangs (#!) only on the first line of the file.
            // Non-first-line #! comments (e.g. commented-out code like
            // `#!self.foo.empty?`) should be flagged.
            if text.starts_with(b"#!") && line == 1 {
                continue;
            }

            // Skip rackup options (#\) on the first line of config.ru files.
            if text.starts_with(b"#\\") && line == 1 && is_config_ru(source) {
                continue;
            }
            let mut diag =
                self.diagnostic(source, line, column, "Missing space after `#`.".to_string());
            if let Some(ref mut corr) = corrections {
                corr.push(crate::correction::Correction {
                    start: start + 1,
                    end: start + 1,
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

fn is_config_ru(source: &SourceFile) -> bool {
    let path = std::path::Path::new(source.path_str());
    path.file_name().and_then(|n| n.to_str()) == Some("config.ru")
}

fn missing_space_after_hash(text: &[u8]) -> bool {
    if text.is_empty() || text[0] != b'#' {
        return false;
    }
    if text.starts_with(b"#++") || text.starts_with(b"#--") {
        return false;
    }

    let hash_run = text.iter().take_while(|&&b| b == b'#').count();
    match text.get(hash_run) {
        None => false,
        Some(b) if b.is_ascii_whitespace() || *b == b'=' => false,
        Some(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(LeadingCommentSpace, "cops/layout/leading_comment_space");
    crate::cop_autocorrect_fixture_tests!(LeadingCommentSpace, "cops/layout/leading_comment_space");

    #[test]
    fn autocorrect_insert_space() {
        let input = b"#comment\n";
        let (_diags, corrections) =
            crate::testutil::run_cop_autocorrect(&LeadingCommentSpace, input);
        assert!(!corrections.is_empty());
        let cs = crate::correction::CorrectionSet::from_vec(corrections);
        let corrected = cs.apply(input);
        assert_eq!(corrected, b"# comment\n");
    }

    #[test]
    fn flags_compact_multi_hash_comments() {
        let diags = crate::testutil::run_cop_full(
            &LeadingCommentSpace,
            b"##patterns += patterns.collect(&:to_s)\n",
        );

        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 0);
    }

    #[test]
    fn allows_multi_hash_comments_with_space() {
        let diags =
            crate::testutil::run_cop_full(&LeadingCommentSpace, b"## section header\n######\n");

        assert!(diags.is_empty());
    }

    #[test]
    fn allows_shebang_on_first_line() {
        let diags =
            crate::testutil::run_cop_full(&LeadingCommentSpace, b"#!/usr/bin/env ruby\nx = 1\n");
        assert!(diags.is_empty());
    }

    #[test]
    fn flags_shebang_not_on_first_line() {
        let diags = crate::testutil::run_cop_full(
            &LeadingCommentSpace,
            b"# comment\n#!/usr/bin/env ruby\n",
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 2);
    }

    #[test]
    fn allows_rackup_options_in_config_ru() {
        let diags = crate::testutil::run_cop_full_internal(
            &LeadingCommentSpace,
            b"#\\ -p 4000\nrun MyApp\n",
            crate::cop::CopConfig::default(),
            "config.ru",
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn flags_double_hash_bang_on_line1() {
        let diags = crate::testutil::run_cop_full(
            &LeadingCommentSpace,
            b"##!/usr/bin/env ruby\nputs 'hello'\n",
        );
        assert_eq!(diags.len(), 1, "should flag ##!/usr/bin/env ruby on line 1");
    }

    #[test]
    fn flags_backslash_comment_in_non_config_ru() {
        let diags = crate::testutil::run_cop_full_internal(
            &LeadingCommentSpace,
            b"#\\ -p 4000\nrun MyApp\n",
            crate::cop::CopConfig::default(),
            "app.rb",
        );
        assert_eq!(diags.len(), 1);
    }
}
