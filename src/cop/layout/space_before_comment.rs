use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct SpaceBeforeComment;

impl Cop for SpaceBeforeComment {
    fn name(&self) -> &'static str {
        "Layout/SpaceBeforeComment"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let bytes = source.as_bytes();

        for comment in parse_result.comments() {
            let loc = comment.location();
            let start = loc.start_offset();

            // Skip if this is the first character on the line (standalone comment)
            if start == 0 {
                continue;
            }
            let prev = bytes[start - 1];
            if prev == b'\n' || prev == b'\r' {
                continue;
            }
            // Skip if preceded only by whitespace (indented standalone comment)
            let mut line_start = start;
            while line_start > 0 && bytes[line_start - 1] != b'\n' {
                line_start -= 1;
            }
            let before_on_line = &bytes[line_start..start];
            if before_on_line.iter().all(|&b| b == b' ' || b == b'\t') {
                continue;
            }

            // Inline comment: check for space before #
            if prev != b' ' && prev != b'\t' {
                let (line, column) = source.offset_to_line_col(start);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Put a space before an end-of-line comment.".to_string(),
                ));
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceBeforeComment, "cops/layout/space_before_comment");
}
