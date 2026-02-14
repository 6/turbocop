use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct AssignmentIndentation;

impl AssignmentIndentation {
    fn check_write(
        &self,
        source: &SourceFile,
        name_offset: usize,
        value: &ruby_prism::Node<'_>,
        width: usize,
    ) -> Vec<Diagnostic> {
        let (name_line, _) = source.offset_to_line_col(name_offset);
        let value_loc = value.location();
        let (value_line, value_col) = source.offset_to_line_col(value_loc.start_offset());

        // Only check when RHS is on a different line
        if value_line == name_line {
            return Vec::new();
        }

        let name_line_bytes = source.lines().nth(name_line - 1).unwrap_or(b"");
        let name_line_indent = indentation_of(name_line_bytes);
        let expected = name_line_indent + width;

        if value_col != expected {
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location {
                    line: value_line,
                    column: value_col,
                },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message:
                    "Indent the first line of the right-hand-side of a multi-line assignment."
                        .to_string(),
            }];
        }

        Vec::new()
    }
}

impl Cop for AssignmentIndentation {
    fn name(&self) -> &'static str {
        "Layout/AssignmentIndentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let width = config
            .options
            .get("IndentationWidth")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        if let Some(n) = node.as_local_variable_write_node() {
            return self.check_write(source, n.name_loc().start_offset(), &n.value(), width);
        }

        if let Some(n) = node.as_instance_variable_write_node() {
            return self.check_write(source, n.name_loc().start_offset(), &n.value(), width);
        }

        if let Some(n) = node.as_class_variable_write_node() {
            return self.check_write(source, n.name_loc().start_offset(), &n.value(), width);
        }

        if let Some(n) = node.as_global_variable_write_node() {
            return self.check_write(source, n.name_loc().start_offset(), &n.value(), width);
        }

        if let Some(n) = node.as_constant_write_node() {
            return self.check_write(source, n.name_loc().start_offset(), &n.value(), width);
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full, run_cop_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &AssignmentIndentation,
            include_bytes!("../../../testdata/cops/layout/assignment_indentation/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &AssignmentIndentation,
            include_bytes!("../../../testdata/cops/layout/assignment_indentation/no_offense.rb"),
        );
    }

    #[test]
    fn single_line_assignment_ignored() {
        let source = b"x = 1\n";
        let diags = run_cop_full(&AssignmentIndentation, source);
        assert!(diags.is_empty());
    }
}
