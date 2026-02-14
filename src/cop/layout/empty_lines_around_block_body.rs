use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundBlockBody;

impl Cop for EmptyLinesAroundBlockBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundBlockBody"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        util::check_empty_lines_around_body(
            self.name(),
            source,
            block_node.opening_loc().start_offset(),
            block_node.closing_loc().start_offset(),
            "block",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        EmptyLinesAroundBlockBody,
        "cops/layout/empty_lines_around_block_body"
    );

    #[test]
    fn single_line_block_no_offense() {
        let src = b"[1, 2, 3].each { |x| puts x }\n";
        let diags = run_cop_full(&EmptyLinesAroundBlockBody, src);
        assert!(diags.is_empty(), "Single-line block should not trigger");
    }

    #[test]
    fn do_end_block_with_blank_lines() {
        let src = b"items.each do |x|\n\n  puts x\n\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundBlockBody, src);
        assert_eq!(diags.len(), 2, "Should flag both beginning and end blank lines");
    }
}
