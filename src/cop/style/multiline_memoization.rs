use crate::cop::node_type::{
    BEGIN_NODE, CALL_OR_WRITE_NODE, CLASS_VARIABLE_OR_WRITE_NODE, CONSTANT_OR_WRITE_NODE,
    GLOBAL_VARIABLE_OR_WRITE_NODE, INDEX_OR_WRITE_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE,
    LOCAL_VARIABLE_OR_WRITE_NODE, PARENTHESES_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks multiline memoization wrapping style (`||=`).
///
/// ## Investigation (2026-03-23)
///
/// **FP root cause:** The multiline check compared the assignment start line
/// against the value end line. For `@x ||=\n  (single_expr if cond)`, the
/// assignment starts on line 1 and the value ends on line 2, so it was
/// incorrectly treated as multiline. RuboCop checks whether the *RHS node
/// itself* spans multiple lines (`rhs.multiline?`), not whether the overall
/// assignment does. Fixed by checking the value node's own start/end lines.
///
/// **FN root cause:** The cop only handled simple variable `||=` nodes
/// (`LocalVariableOrWriteNode`, `InstanceVariableOrWriteNode`, etc.) but
/// missed `CallOrWriteNode` (`foo.bar ||=`) and `IndexOrWriteNode`
/// (`foo["key"] ||=`). These are common in real-world code (e.g.,
/// `@info["exif"] ||= (...)`). Fixed by adding both node types.
pub struct MultilineMemoization;

impl Cop for MultilineMemoization {
    fn name(&self) -> &'static str {
        "Style/MultilineMemoization"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BEGIN_NODE,
            CALL_OR_WRITE_NODE,
            CLASS_VARIABLE_OR_WRITE_NODE,
            CONSTANT_OR_WRITE_NODE,
            GLOBAL_VARIABLE_OR_WRITE_NODE,
            INDEX_OR_WRITE_NODE,
            INSTANCE_VARIABLE_OR_WRITE_NODE,
            LOCAL_VARIABLE_OR_WRITE_NODE,
            PARENTHESES_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "keyword");

        // Extract (assignment location, value) from any kind of ||= node
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
        } else if let Some(n) = node.as_call_or_write_node() {
            (n.location(), n.value())
        } else if let Some(n) = node.as_index_or_write_node() {
            (n.location(), n.value())
        } else {
            return;
        };

        // Check if the VALUE NODE ITSELF spans multiple lines.
        // RuboCop uses `rhs.multiline?` which checks the RHS node's own span.
        // This avoids false positives where the assignment operator is on a
        // different line than the value but the value itself is single-line.
        let value_loc = value.location();
        let value_start_line = source.offset_to_line_col(value_loc.start_offset()).0;
        let value_end_offset = value_loc.start_offset() + value_loc.as_slice().len();
        let value_end_line = source
            .offset_to_line_col(value_end_offset.saturating_sub(1))
            .0;

        if value_start_line == value_end_line {
            // Value is single-line — not a multiline memoization
            return;
        }

        // It's multiline. Check the wrapping style.
        if enforced_style == "keyword" {
            // keyword style: should use begin..end, not parentheses
            if value.as_parentheses_node().is_some() {
                let (line, column) = source.offset_to_line_col(assign_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Wrap multiline memoization blocks in `begin` and `end`.".to_string(),
                ));
            }
        } else if enforced_style == "braces" {
            // braces style: should use parentheses, not begin..end
            if value.as_begin_node().is_some() {
                let (line, column) = source.offset_to_line_col(assign_loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Wrap multiline memoization blocks in `(` and `)`.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineMemoization, "cops/style/multiline_memoization");
}
