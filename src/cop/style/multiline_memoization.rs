use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BEGIN_NODE, CLASS_VARIABLE_OR_WRITE_NODE, CONSTANT_OR_WRITE_NODE, GLOBAL_VARIABLE_OR_WRITE_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_OR_WRITE_NODE, PARENTHESES_NODE};

pub struct MultilineMemoization;

impl Cop for MultilineMemoization {
    fn name(&self) -> &'static str {
        "Style/MultilineMemoization"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, CLASS_VARIABLE_OR_WRITE_NODE, CONSTANT_OR_WRITE_NODE, GLOBAL_VARIABLE_OR_WRITE_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_OR_WRITE_NODE, PARENTHESES_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "keyword");

        // Extract (location, value) from any kind of ||= node
        let (assign_loc, value) = if let Some(n) = node.as_local_variable_or_write_node() {
            (n.location(), n.value())
        } else if let Some(n) = node.as_instance_variable_or_write_node() {
            (n.location(), n.value())
        } else if let Some(n) = node.as_class_variable_or_write_node() {
            (n.location(), n.value())
        } else if let Some(n) = node.as_global_variable_or_write_node() {
            (n.location(), n.value())
        } else if let Some(n) = node.as_constant_or_write_node() {
            (n.location(), n.value())
        } else {
            return Vec::new();
        };

        // Check if the value spans multiple lines
        let assign_line = source.offset_to_line_col(assign_loc.start_offset()).0;
        let value_end_offset = value.location().start_offset() + value.location().as_slice().len();
        let value_end_line = source.offset_to_line_col(value_end_offset.saturating_sub(1)).0;

        if assign_line == value_end_line {
            // Single line - not a multiline memoization
            return Vec::new();
        }

        // It's multiline. Check the wrapping style.
        if enforced_style == "keyword" {
            // keyword style: should use begin..end, not parentheses
            if value.as_parentheses_node().is_some() {
                let (line, column) = source.offset_to_line_col(assign_loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Wrap multiline memoization blocks in `begin` and `end`.".to_string(),
                )];
            }
        } else if enforced_style == "braces" {
            // braces style: should use parentheses, not begin..end
            if value.as_begin_node().is_some() {
                let (line, column) = source.offset_to_line_col(assign_loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Wrap multiline memoization blocks in `(` and `)`.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineMemoization, "cops/style/multiline_memoization");
}
