use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundClassBody;

impl Cop for EmptyLinesAroundClassBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundClassBody"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        util::check_empty_lines_around_body(
            self.name(),
            source,
            class_node.class_keyword_loc().start_offset(),
            class_node.end_keyword_loc().start_offset(),
            "class",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        EmptyLinesAroundClassBody,
        "cops/layout/empty_lines_around_class_body"
    );

    #[test]
    fn single_line_class_no_offense() {
        let src = b"class Foo; end\n";
        let diags = run_cop_full(&EmptyLinesAroundClassBody, src);
        assert!(diags.is_empty(), "Single-line class should not trigger");
    }

    #[test]
    fn blank_line_at_both_ends() {
        let src = b"class Foo\n\n  def bar; end\n\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundClassBody, src);
        assert_eq!(diags.len(), 2, "Should flag both beginning and end blank lines");
    }
}
