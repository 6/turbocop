use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LeadingCommentSpace;

impl Cop for LeadingCommentSpace {
    fn name(&self) -> &'static str {
        "Layout/LeadingCommentSpace"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _allow_doxygen = config.get_bool("AllowDoxygenCommentStyle", false);
        let _allow_gemfile_ruby = config.get_bool("AllowGemfileRubyComment", false);
        let _allow_rbs_inline = config.get_bool("AllowRBSInlineAnnotation", false);
        let _allow_steep = config.get_bool("AllowSteepAnnotation", false);
        let bytes = source.as_bytes();
        let mut diagnostics = Vec::new();

        for comment in parse_result.comments() {
            let loc = comment.location();
            let start = loc.start_offset();
            let end = loc.end_offset();
            let text = &bytes[start..end];

            // Must start with #
            if text.is_empty() || text[0] != b'#' {
                continue;
            }

            // Skip shebangs (#!)
            if text.len() > 1 && text[1] == b'!' {
                continue;
            }

            // Skip empty comments (just #)
            if text.len() == 1 {
                continue;
            }

            // After #, the next char should be a space, or end of comment
            let after_hash = text[1];
            if after_hash != b' ' && after_hash != b'\t' {
                // Check for ## (allow ##word for doxygen if enabled)
                if after_hash == b'#' {
                    continue;
                }

                // Skip RDoc toggle comments: #++ and #--
                if text.len() > 2 && (after_hash == b'+' || after_hash == b'-') && text[2] == after_hash {
                    continue;
                }

                // Skip #= (RDoc =begin/=end style)
                if after_hash == b'=' {
                    continue;
                }
                let (line, column) = source.offset_to_line_col(start);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Missing space after `#`.".to_string(),
                ));
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(LeadingCommentSpace, "cops/layout/leading_comment_space");
}
