use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundBeginBody;

impl Cop for EmptyLinesAroundBeginBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundBeginBody"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Only check explicit begin..end blocks (BeginNode in Prism)
        let begin_node = match node.as_begin_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Must have begin and end keywords
        let begin_keyword_loc = match begin_node.begin_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };
        let end_keyword_loc = match begin_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        util::check_empty_lines_around_body(
            self.name(),
            source,
            begin_keyword_loc.start_offset(),
            end_keyword_loc.start_offset(),
            "`begin`",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAroundBeginBody,
        "cops/layout/empty_lines_around_begin_body"
    );
}
