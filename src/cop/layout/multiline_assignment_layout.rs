use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BEGIN_NODE, BLOCK_NODE, CASE_MATCH_NODE, CASE_NODE, CLASS_NODE, CLASS_VARIABLE_WRITE_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE, LAMBDA_NODE, LOCAL_VARIABLE_WRITE_NODE, MODULE_NODE, UNLESS_NODE};

pub struct MultilineAssignmentLayout;

/// Check if a node represents one of the supported types for this cop.
fn is_supported_type(node: &ruby_prism::Node<'_>, supported_types: &[String]) -> bool {
    for t in supported_types {
        let matches = match t.as_str() {
            "if" => node.as_if_node().is_some() || node.as_unless_node().is_some(),
            "case" => node.as_case_node().is_some() || node.as_case_match_node().is_some(),
            "class" => node.as_class_node().is_some(),
            "module" => node.as_module_node().is_some(),
            "kwbegin" => node.as_begin_node().is_some(),
            "block" => node.as_block_node().is_some() || node.as_lambda_node().is_some(),
            _ => false,
        };
        if matches {
            return true;
        }
    }
    false
}

/// Find the `=` sign byte offset between the name end and the value start
/// by scanning the raw bytes. Returns None if not found.
fn find_eq_offset(source: &SourceFile, name_end: usize, value_start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    for i in name_end..value_start.min(bytes.len()) {
        if bytes[i] == b'=' {
            // Make sure it's a standalone `=` and not `==`
            if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                continue;
            }
            return Some(i);
        }
    }
    None
}

impl Cop for MultilineAssignmentLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineAssignmentLayout"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BEGIN_NODE, BLOCK_NODE, CASE_MATCH_NODE, CASE_NODE, CLASS_NODE, CLASS_VARIABLE_WRITE_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE, LAMBDA_NODE, LOCAL_VARIABLE_WRITE_NODE, MODULE_NODE, UNLESS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "new_line");
        let supported_types = config
            .get_string_array("SupportedTypes")
            .unwrap_or_else(|| {
                vec![
                    "block".to_string(),
                    "case".to_string(),
                    "class".to_string(),
                    "if".to_string(),
                    "kwbegin".to_string(),
                    "module".to_string(),
                ]
            });

        // Extract (name_end_offset, value_node) from various assignment types
        let (name_end, value) = if let Some(asgn) = node.as_local_variable_write_node() {
            (asgn.name_loc().end_offset(), asgn.value())
        } else if let Some(asgn) = node.as_instance_variable_write_node() {
            (asgn.name_loc().end_offset(), asgn.value())
        } else if let Some(asgn) = node.as_constant_write_node() {
            (asgn.name_loc().end_offset(), asgn.value())
        } else if let Some(asgn) = node.as_class_variable_write_node() {
            (asgn.name_loc().end_offset(), asgn.value())
        } else if let Some(asgn) = node.as_global_variable_write_node() {
            (asgn.name_loc().end_offset(), asgn.value())
        } else {
            return;
        };

        if !is_supported_type(&value, &supported_types) {
            return;
        }

        let (value_start_line, _) = source.offset_to_line_col(value.location().start_offset());
        let (value_end_line, _) = source.offset_to_line_col(
            value.location().end_offset().saturating_sub(1),
        );

        // Only check multi-line RHS
        if value_start_line == value_end_line {
            return;
        }

        // Find the `=` sign between the name and the value
        let eq_offset = match find_eq_offset(source, name_end, value.location().start_offset()) {
            Some(o) => o,
            None => return,
        };

        let (eq_line, _) = source.offset_to_line_col(eq_offset);
        let same_line = eq_line == value_start_line;
        let (node_line, node_col) = source.offset_to_line_col(node.location().start_offset());

        match enforced_style {
            "new_line" => {
                if same_line {
                    diagnostics.push(self.diagnostic(
                        source,
                        node_line,
                        node_col,
                        "Right hand side of multi-line assignment is on the same line as the assignment operator `=`.".to_string(),
                    ));
                }
            }
            "same_line" => {
                if !same_line {
                    diagnostics.push(self.diagnostic(
                        source,
                        node_line,
                        node_col,
                        "Right hand side of multi-line assignment is not on the same line as the assignment operator `=`.".to_string(),
                    ));
                }
            }
            _ => {}
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineAssignmentLayout,
        "cops/layout/multiline_assignment_layout"
    );
}
