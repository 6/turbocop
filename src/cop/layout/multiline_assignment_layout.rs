use crate::cop::node_type::{
    BEGIN_NODE, BLOCK_NODE, CALL_AND_WRITE_NODE, CALL_NODE, CALL_OPERATOR_WRITE_NODE,
    CALL_OR_WRITE_NODE, CASE_NODE, CLASS_NODE, CLASS_VARIABLE_AND_WRITE_NODE,
    CLASS_VARIABLE_OPERATOR_WRITE_NODE, CLASS_VARIABLE_OR_WRITE_NODE, CLASS_VARIABLE_WRITE_NODE,
    CONSTANT_AND_WRITE_NODE, CONSTANT_OPERATOR_WRITE_NODE, CONSTANT_OR_WRITE_NODE,
    CONSTANT_PATH_AND_WRITE_NODE, CONSTANT_PATH_OPERATOR_WRITE_NODE, CONSTANT_PATH_OR_WRITE_NODE,
    CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_AND_WRITE_NODE,
    GLOBAL_VARIABLE_OPERATOR_WRITE_NODE, GLOBAL_VARIABLE_OR_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE,
    IF_NODE, INDEX_AND_WRITE_NODE, INDEX_OPERATOR_WRITE_NODE, INDEX_OR_WRITE_NODE,
    INSTANCE_VARIABLE_AND_WRITE_NODE, INSTANCE_VARIABLE_OPERATOR_WRITE_NODE,
    INSTANCE_VARIABLE_OR_WRITE_NODE, INSTANCE_VARIABLE_WRITE_NODE, LAMBDA_NODE,
    LOCAL_VARIABLE_AND_WRITE_NODE, LOCAL_VARIABLE_OPERATOR_WRITE_NODE,
    LOCAL_VARIABLE_OR_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE, MODULE_NODE, MULTI_WRITE_NODE,
    UNLESS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-04-02)
///
/// RuboCop handles two shapes that Prism exposes differently here:
///
/// - Setter and index assignments like `foo.bar = if ...` and
///   `hash[:key] = case ...` are plain `send` nodes in RuboCop and
///   attribute-write `CallNode`s in Prism, with the assigned value as the last
///   argument.
/// - For block RHS values, RuboCop keeps the enclosing call expression as the
///   RHS start line, but its `single_line?` logic is based on the block
///   delimiters. Prism keeps `do/end` and `{}` blocks attached to a `CallNode`,
///   so treating either only the whole call or only the block location causes
///   mismatches.
///
/// Fixes applied:
/// - Added attribute-write `CallNode` handling so setter/index `=` assignments
///   participate in the same multiline RHS check as other assignments.
/// - For call-with-block RHS values, keep the call's start line for the
///   same-line check, but use the attached block delimiter span to decide
///   whether the RHS counts as multiline. This matches RuboCop's behavior for
///   single-line `{}` blocks versus multiline `do/end` or multiline `{}` blocks.
/// - Added Prism compound-assignment and multi-assignment node handling so
///   `||=`, `&&=`, `+=`, and `masgn` variants follow the same RHS layout rules.
/// - Excluded numbered-parameter / implicit-`it` blocks (`numblock` / `itblock`
///   in RuboCop, e.g. `_1` or `it`) from the default `block` support. Prism
///   reports them as `BlockNode`s with `NumberedParametersNode` or
///   `ItParametersNode`, but RuboCop does not treat them as supported `block`
///   RHS values here.
pub struct MultilineAssignmentLayout;

/// Check if a node represents one of the supported types for this cop.
fn is_supported_type(node: &ruby_prism::Node<'_>, supported_types: &[String]) -> bool {
    for t in supported_types {
        match t.as_str() {
            "if" if node.as_if_node().is_some() || node.as_unless_node().is_some() => {
                return true;
            }
            "case" if node.as_case_node().is_some() => {
                return true;
            }
            "class" if node.as_class_node().is_some() => return true,
            "module" if node.as_module_node().is_some() => return true,
            "kwbegin" if node.as_begin_node().is_some() => return true,
            "block" => {
                if node.as_block_node().is_some() || node.as_lambda_node().is_some() {
                    return true;
                }

                if let Some(call) = node.as_call_node() {
                    if let Some(block) = call.block() {
                        let is_special_block = block
                            .as_block_node()
                            .and_then(|block| block.parameters())
                            .is_some_and(|params| {
                                params.as_numbered_parameters_node().is_some()
                                    || params.as_it_parameters_node().is_some()
                            });

                        if !is_special_block {
                            return true;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    false
}

/// Mirror RuboCop's `single_line?` handling for block RHS values.
///
/// For call-with-block expressions, RuboCop keeps the call expression as the
/// RHS start line, but treats `{ ... }` blocks with both delimiters on one line
/// as single-line even when the receiver chain spans multiple lines.
fn rhs_is_multiline(
    source: &SourceFile,
    value: &ruby_prism::Node<'_>,
    supported_types: &[String],
) -> bool {
    if supported_types.iter().any(|t| t == "block") {
        if let Some(block) = value.as_call_node().and_then(|call| call.block()) {
            let (block_start_line, _) = source.offset_to_line_col(block.location().start_offset());
            let (block_end_line, _) =
                source.offset_to_line_col(block.location().end_offset().saturating_sub(1));

            return block_start_line != block_end_line;
        }
    }

    let (value_start_line, _) = source.offset_to_line_col(value.location().start_offset());
    let (value_end_line, _) =
        source.offset_to_line_col(value.location().end_offset().saturating_sub(1));

    value_start_line != value_end_line
}

/// Find the assignment operator byte offset by scanning backwards from the RHS.
/// This catches both `=` and `||=` forms while preferring the last operator
/// before the value start.
fn find_eq_offset(
    source: &SourceFile,
    assignment_start: usize,
    value_start: usize,
) -> Option<usize> {
    let bytes = source.as_bytes();
    let end = value_start.min(bytes.len());
    for i in (assignment_start..end).rev() {
        if bytes[i] != b'=' {
            continue;
        }

        // Skip comparison operators like `==`/`===` by ignoring both sides.
        if i + 1 < end && bytes[i + 1] == b'=' {
            continue;
        }
        if i > assignment_start && bytes[i - 1] == b'=' {
            continue;
        }

        return Some(i);
    }
    None
}

fn assignment_start_and_value<'a>(
    node: &'a ruby_prism::Node<'a>,
) -> Option<(usize, ruby_prism::Node<'a>)> {
    if let Some(asgn) = node.as_local_variable_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_local_variable_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_local_variable_and_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_local_variable_operator_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_instance_variable_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_instance_variable_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_instance_variable_and_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_instance_variable_operator_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_and_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_operator_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_path_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_path_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_path_and_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_constant_path_operator_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_class_variable_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_class_variable_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_class_variable_and_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_class_variable_operator_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_global_variable_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_global_variable_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_global_variable_and_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_global_variable_operator_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_multi_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(call) = node.as_call_node() {
        if !call.is_attribute_write() {
            return None;
        }

        let args = call.arguments()?;
        let value = args.arguments().iter().last()?;
        Some((call.location().start_offset(), value))
    } else if let Some(asgn) = node.as_call_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_call_and_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_call_operator_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_index_or_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else if let Some(asgn) = node.as_index_and_write_node() {
        Some((asgn.location().start_offset(), asgn.value()))
    } else {
        node.as_index_operator_write_node()
            .map(|asgn| (asgn.location().start_offset(), asgn.value()))
    }
}

impl Cop for MultilineAssignmentLayout {
    fn name(&self) -> &'static str {
        "Layout/MultilineAssignmentLayout"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            BEGIN_NODE,
            BLOCK_NODE,
            CALL_AND_WRITE_NODE,
            CALL_NODE,
            CALL_OPERATOR_WRITE_NODE,
            CALL_OR_WRITE_NODE,
            CASE_NODE,
            CLASS_NODE,
            CLASS_VARIABLE_AND_WRITE_NODE,
            CLASS_VARIABLE_OPERATOR_WRITE_NODE,
            CLASS_VARIABLE_OR_WRITE_NODE,
            CLASS_VARIABLE_WRITE_NODE,
            CONSTANT_AND_WRITE_NODE,
            CONSTANT_OPERATOR_WRITE_NODE,
            CONSTANT_OR_WRITE_NODE,
            CONSTANT_PATH_AND_WRITE_NODE,
            CONSTANT_PATH_OPERATOR_WRITE_NODE,
            CONSTANT_PATH_OR_WRITE_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            GLOBAL_VARIABLE_AND_WRITE_NODE,
            GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
            GLOBAL_VARIABLE_OR_WRITE_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            IF_NODE,
            INDEX_AND_WRITE_NODE,
            INDEX_OPERATOR_WRITE_NODE,
            INDEX_OR_WRITE_NODE,
            INSTANCE_VARIABLE_AND_WRITE_NODE,
            INSTANCE_VARIABLE_OPERATOR_WRITE_NODE,
            INSTANCE_VARIABLE_OR_WRITE_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LAMBDA_NODE,
            LOCAL_VARIABLE_AND_WRITE_NODE,
            LOCAL_VARIABLE_OPERATOR_WRITE_NODE,
            LOCAL_VARIABLE_OR_WRITE_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            MODULE_NODE,
            MULTI_WRITE_NODE,
            UNLESS_NODE,
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

        let (assignment_start, value) = match assignment_start_and_value(node) {
            Some(parts) => parts,
            None => return,
        };

        if !is_supported_type(&value, &supported_types) {
            return;
        }

        let (value_start_line, _) = source.offset_to_line_col(value.location().start_offset());

        // Only check RHS values that RuboCop considers multi-line.
        if !rhs_is_multiline(source, &value, &supported_types) {
            return;
        }

        let eq_offset =
            match find_eq_offset(source, assignment_start, value.location().start_offset()) {
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
