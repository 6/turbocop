use crate::cop::shared::node_type::{
    AND_NODE, CALL_NODE, CLASS_VARIABLE_READ_NODE, CLASS_VARIABLE_WRITE_NODE,
    INSTANCE_VARIABLE_READ_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_READ_NODE,
    LOCAL_VARIABLE_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-27)
///
/// Corpus oracle reported FP=0, FN=191.
///
/// The dominant FN bucket was `x = x || y` patterns in local assignments,
/// including modifier-`if` forms such as
/// `response = response || fallback if condition`.
///
/// Root cause: the cop only checked arithmetic `CallNode`s and boolean
/// `AndNode`s. Prism parses `||` as `OrNode`, so every `||=` shorthand case was
/// skipped.
///
/// Fix: treat `OrNode` the same as `AndNode` by comparing the left operand with
/// the assignment target and deriving the emitted operator from `operator_loc`.
pub struct SelfAssignment;

const SELF_ASSIGN_OPS: &[&[u8]] = &[
    b"+", b"-", b"*", b"**", b"/", b"%", b"^", b"<<", b">>", b"|", b"&",
];

impl SelfAssignment {
    fn get_write_name(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
        if let Some(lv) = node.as_local_variable_write_node() {
            return Some(lv.name().as_slice().to_vec());
        }
        if let Some(iv) = node.as_instance_variable_write_node() {
            return Some(iv.name().as_slice().to_vec());
        }
        if let Some(cv) = node.as_class_variable_write_node() {
            return Some(cv.name().as_slice().to_vec());
        }
        None
    }

    fn get_read_name(node: &ruby_prism::Node<'_>) -> Option<Vec<u8>> {
        if let Some(lv) = node.as_local_variable_read_node() {
            return Some(lv.name().as_slice().to_vec());
        }
        if let Some(iv) = node.as_instance_variable_read_node() {
            return Some(iv.name().as_slice().to_vec());
        }
        if let Some(cv) = node.as_class_variable_read_node() {
            return Some(cv.name().as_slice().to_vec());
        }
        None
    }

    fn check_boolean_assignment(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        value: &ruby_prism::Node<'_>,
        write_name: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let (left, op_loc) = if let Some(and_node) = value.as_and_node() {
            (and_node.left(), and_node.operator_loc())
        } else if let Some(or_node) = value.as_or_node() {
            (or_node.left(), or_node.operator_loc())
        } else {
            return;
        };

        if let Some(read_name) = Self::get_read_name(&left) {
            if read_name.as_slice() == write_name {
                let op = std::str::from_utf8(op_loc.as_slice()).unwrap_or("&&");
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Use self-assignment shorthand `{}=`.", op),
                ));
            }
        }
    }
}

impl Cop for SelfAssignment {
    fn name(&self) -> &'static str {
        "Style/SelfAssignment"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            AND_NODE,
            CALL_NODE,
            CLASS_VARIABLE_READ_NODE,
            CLASS_VARIABLE_WRITE_NODE,
            INSTANCE_VARIABLE_READ_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LOCAL_VARIABLE_READ_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let write_name = match Self::get_write_name(node) {
            Some(n) => n,
            None => return,
        };

        // Get the value being assigned
        let value = if let Some(lv) = node.as_local_variable_write_node() {
            lv.value()
        } else if let Some(iv) = node.as_instance_variable_write_node() {
            iv.value()
        } else if let Some(cv) = node.as_class_variable_write_node() {
            cv.value()
        } else {
            return;
        };

        // Check for `x = x op y` pattern
        if let Some(call) = value.as_call_node() {
            let method_name = call.name();
            let method_bytes = method_name.as_slice();

            if !SELF_ASSIGN_OPS.contains(&method_bytes) {
                return;
            }

            // Must have exactly one argument
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    return;
                }
            } else {
                return;
            }

            // Receiver must be the same variable
            if let Some(receiver) = call.receiver() {
                if let Some(read_name) = Self::get_read_name(&receiver) {
                    if read_name == write_name {
                        let op = std::str::from_utf8(method_bytes).unwrap_or("+");
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Use self-assignment shorthand `{}=`.", op),
                        ));
                    }
                }
            }
        }

        // Check for boolean operators: `x = x && y` and `x = x || y`
        self.check_boolean_assignment(source, node, &value, &write_name, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SelfAssignment, "cops/style/self_assignment");
}
