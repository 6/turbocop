use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CaseIndentation;

impl Cop for CaseIndentation {
    fn name(&self) -> &'static str {
        "Layout/CaseIndentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let case_node = match node.as_case_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let case_loc = case_node.case_keyword_loc();
        let (_, case_col) = source.offset_to_line_col(case_loc.start_offset());

        let mut diagnostics = Vec::new();

        for condition in case_node.conditions().iter() {
            if let Some(when_node) = condition.as_when_node() {
                let when_loc = when_node.keyword_loc();
                let (when_line, when_col) = source.offset_to_line_col(when_loc.start_offset());

                if when_col != case_col {
                    diagnostics.push(self.diagnostic(
                        source,
                        when_line,
                        when_col,
                        "Indent `when` as deep as `case`.".to_string(),
                    ));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(CaseIndentation, "cops/layout/case_indentation");

    #[test]
    fn nested_case_respects_own_indent() {
        let src = b"case x\nwhen 1\n  case y\n  when :a\n    puts :a\n  end\nend\n";
        let diags = run_cop_full(&CaseIndentation, src);
        assert!(diags.is_empty(), "Properly indented nested case should not trigger");
    }

    #[test]
    fn multiple_when_some_misaligned() {
        let src = b"case x\nwhen 1\n  puts 1\n  when 2\n  puts 2\nend\n";
        let diags = run_cop_full(&CaseIndentation, src);
        assert_eq!(diags.len(), 1, "Only the misaligned when should trigger");
        assert_eq!(diags[0].location.line, 4);
        assert_eq!(diags[0].location.column, 2);
    }
}
