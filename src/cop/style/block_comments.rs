use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Style/BlockComments: Do not use block comments (`=begin`/`=end`).
///
/// Investigation: 19 FPs were caused by `=begin` appearing inside heredoc
/// strings (e.g., test files for rdoc/yard/coderay parsers). Fixed by
/// switching from `check_lines` to `check_source` to access the CodeMap
/// and skip `=begin` lines that fall within heredoc byte ranges.
///
/// Additional 4 FPs from `=begin` appearing after `__END__` data section
/// markers. RuboCop stops parsing at `__END__`, so block comments in the
/// data section are not flagged. Fixed by using `is_not_string()` which
/// covers heredocs, string literals, and `__END__` data sections.
pub struct BlockComments;

impl Cop for BlockComments {
    fn name(&self) -> &'static str {
        "Style/BlockComments"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        for (i, line) in source.lines().enumerate() {
            // =begin must be at the start of a line
            if line.starts_with(b"=begin") && (line.len() == 6 || line[6].is_ascii_whitespace()) {
                // Skip =begin inside heredocs (e.g., test files for rdoc/yard)
                // or after __END__ data section marker (not real code).
                // is_not_string() returns false for heredocs, strings, and __END__ data.
                if let Some(offset) = source.line_col_to_offset(i + 1, 0) {
                    if !code_map.is_not_string(offset) {
                        continue;
                    }
                }
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    0,
                    "Do not use block comments.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BlockComments, "cops/style/block_comments");
}
