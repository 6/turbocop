use crate::cop::util::indentation_of;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CLASS_VARIABLE_WRITE_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE};

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
            return vec![self.diagnostic(
                source,
                value_line,
                value_col,
                "Indent the first line of the right-hand-side of a multi-line assignment."
                    .to_string(),
            )];
        }

        Vec::new()
    }
}

impl Cop for AssignmentIndentation {
    fn name(&self) -> &'static str {
        "Layout/AssignmentIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CLASS_VARIABLE_WRITE_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let width = config.get_usize("IndentationWidth", 2);

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
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(AssignmentIndentation, "cops/layout/assignment_indentation");

    #[test]
    fn single_line_assignment_ignored() {
        let source = b"x = 1\n";
        let diags = run_cop_full(&AssignmentIndentation, source);
        assert!(diags.is_empty());
    }
}
