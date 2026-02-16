use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

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
}

impl Cop for SelfAssignment {
    fn name(&self) -> &'static str {
        "Style/SelfAssignment"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let write_name = match Self::get_write_name(node) {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Get the value being assigned
        let value = if let Some(lv) = node.as_local_variable_write_node() {
            lv.value()
        } else if let Some(iv) = node.as_instance_variable_write_node() {
            iv.value()
        } else if let Some(cv) = node.as_class_variable_write_node() {
            cv.value()
        } else {
            return Vec::new();
        };

        // Check for `x = x op y` pattern
        if let Some(call) = value.as_call_node() {
            let method_name = call.name();
            let method_bytes = method_name.as_slice();

            if !SELF_ASSIGN_OPS.contains(&method_bytes) {
                return Vec::new();
            }

            // Must have exactly one argument
            if let Some(args) = call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }

            // Receiver must be the same variable
            if let Some(receiver) = call.receiver() {
                if let Some(read_name) = Self::get_read_name(&receiver) {
                    if read_name == write_name {
                        let op = std::str::from_utf8(method_bytes).unwrap_or("+");
                        let loc = node.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            format!("Use self-assignment shorthand `{}=`.", op),
                        )];
                    }
                }
            }
        }

        // Check for boolean operators: `x = x && y`
        if let Some(and_node) = value.as_and_node() {
            let left = and_node.left();
            if let Some(read_name) = Self::get_read_name(&left) {
                if read_name == write_name {
                    let op_loc = and_node.operator_loc();
                    let op = std::str::from_utf8(op_loc.as_slice()).unwrap_or("&&");
                    let loc = node.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Use self-assignment shorthand `{}=`.", op),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SelfAssignment, "cops/style/self_assignment");
}
