use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::BEGIN_NODE;

pub struct EmptyLinesAroundBeginBody;

impl Cop for EmptyLinesAroundBeginBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundBeginBody"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Only check explicit begin..end blocks (BeginNode in Prism)
        let begin_node = match node.as_begin_node() {
            Some(b) => b,
            None => return,
        };

        // Must have begin and end keywords
        let begin_keyword_loc = match begin_node.begin_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };
        let end_keyword_loc = match begin_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        diagnostics.extend(util::check_empty_lines_around_body(
            self.name(),
            source,
            begin_keyword_loc.start_offset(),
            end_keyword_loc.start_offset(),
            "`begin`",
        ));
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
