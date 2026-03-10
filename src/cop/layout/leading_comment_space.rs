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

        for comment in parse_result.comments() {
            let loc = comment.location();
            let start = loc.start_offset();
            let end = loc.end_offset();
            let text = &bytes[start..end];

            if !missing_space_after_hash(text) {
                continue;
            }

            // Skip shebangs (#!)
            if text.starts_with(b"#!") {
                continue;
            }

            let (line, column) = source.offset_to_line_col(start);
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
}
