use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
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
                    diagnostics.push(Diagnostic {
                        path: source.path_str().to_string(),
                        location: Location {
                            line: when_line,
                            column: when_col,
                        },
                        severity: Severity::Convention,
                        cop_name: self.name().to_string(),
                        message: "Indent `when` as deep as `case`.".to_string(),
                    });
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &CaseIndentation,
            include_bytes!("../../../testdata/cops/layout/case_indentation/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &CaseIndentation,
            include_bytes!("../../../testdata/cops/layout/case_indentation/no_offense.rb"),
        );
    }

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
