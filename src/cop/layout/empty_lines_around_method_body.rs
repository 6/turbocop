use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::DEF_NODE;

pub struct EmptyLinesAroundMethodBody;

impl Cop for EmptyLinesAroundMethodBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundMethodBody"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Skip endless methods (no end keyword)
        let end_loc = match def_node.end_keyword_loc() {
            Some(loc) => loc,
            None => return,
        };

        diagnostics.extend(util::check_empty_lines_around_body_with_corrections(
            self.name(),
            source,
            def_node.def_keyword_loc().start_offset(),
            end_loc.start_offset(),
            "method",
            corrections,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        EmptyLinesAroundMethodBody,
        "cops/layout/empty_lines_around_method_body"
    );
    crate::cop_autocorrect_fixture_tests!(
        EmptyLinesAroundMethodBody,
        "cops/layout/empty_lines_around_method_body"
    );

    #[test]
    fn single_line_def_no_offense() {
        let src = b"def foo; 42; end\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert!(diags.is_empty(), "Single-line def should not trigger");
    }

    #[test]
    fn endless_method_no_offense() {
        let src = b"def foo = 42\n";
        let diags = run_cop_full(&EmptyLinesAroundMethodBody, src);
        assert!(diags.is_empty(), "Endless method should not trigger");
    }
}
