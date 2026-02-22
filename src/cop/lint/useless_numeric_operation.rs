use crate::cop::node_type::{
    CALL_NODE, CLASS_VARIABLE_OPERATOR_WRITE_NODE, FLOAT_NODE, GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
    INSTANCE_VARIABLE_OPERATOR_WRITE_NODE, INTEGER_NODE, LOCAL_VARIABLE_OPERATOR_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UselessNumericOperation;

const MSG: &str = "Do not apply inconsequential numeric operations to variables.";

impl Cop for UselessNumericOperation {
    fn name(&self) -> &'static str {
        "Lint/UselessNumericOperation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_VARIABLE_OPERATOR_WRITE_NODE,
            FLOAT_NODE,
            GLOBAL_VARIABLE_OPERATOR_WRITE_NODE,
            INSTANCE_VARIABLE_OPERATOR_WRITE_NODE,
            INTEGER_NODE,
            LOCAL_VARIABLE_OPERATOR_WRITE_NODE,
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
        // Check for binary operator calls: x + 0, x - 0, x * 1, x / 1, x ** 1
        // RuboCop only matches `(call (call nil? $_) $_ (int $_))`, meaning the
        // receiver must be a bare method call (no receiver of its own). This
        // corresponds to simple method-name references (x + 0), NOT local
        // variables, instance variables, constants, or chained calls.
        if let Some(call) = node.as_call_node() {
            let method = call.name().as_slice();

            // Check receiver exists and is a bare method call (no receiver of its own)
            let recv = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            // RuboCop's pattern: (call (call nil? $_) $_ (int $_))
            // The receiver must be a CallNode with no receiver (bare method call).
            let is_bare_method_call = match recv.as_call_node() {
                Some(recv_call) => recv_call.receiver().is_none(),
                None => false,
            };
            if !is_bare_method_call {
                return;
            }

            let arguments = match call.arguments() {
                Some(a) => a,
                None => return,
            };

            let args = arguments.arguments();
            if args.len() != 1 {
                return;
            }

            let arg = args.iter().next().unwrap();

            let is_useless = match method {
                b"+" | b"-" => is_zero(&arg, source),
                b"*" | b"/" | b"**" => is_one(&arg, source),
                _ => false,
            };

            if is_useless {
                let loc = call.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
            }
        }

        // Check for operator assignment: x += 0, x -= 0, x *= 1, x /= 1, x **= 1
        if let Some(op_assign) = node.as_local_variable_operator_write_node() {
            let operator = op_assign.binary_operator().as_slice();
            let value = op_assign.value();

            let is_useless = match operator {
                b"+" | b"-" => is_zero(&value, source),
                b"*" | b"/" | b"**" => is_one(&value, source),
                _ => false,
            };

            if is_useless {
                let loc = op_assign.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
            }
        }

        // Also handle instance variable operator writes
        if let Some(op_assign) = node.as_instance_variable_operator_write_node() {
            let operator = op_assign.binary_operator().as_slice();
            let value = op_assign.value();

            let is_useless = match operator {
                b"+" | b"-" => is_zero(&value, source),
                b"*" | b"/" | b"**" => is_one(&value, source),
                _ => false,
            };

            if is_useless {
                let loc = op_assign.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
            }
        }

        // Global variable operator writes
        if let Some(op_assign) = node.as_global_variable_operator_write_node() {
            let operator = op_assign.binary_operator().as_slice();
            let value = op_assign.value();

            let is_useless = match operator {
                b"+" | b"-" => is_zero(&value, source),
                b"*" | b"/" | b"**" => is_one(&value, source),
                _ => false,
            };

            if is_useless {
                let loc = op_assign.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
            }
        }

        // Class variable operator writes
        if let Some(op_assign) = node.as_class_variable_operator_write_node() {
            let operator = op_assign.binary_operator().as_slice();
            let value = op_assign.value();

            let is_useless = match operator {
                b"+" | b"-" => is_zero(&value, source),
                b"*" | b"/" | b"**" => is_one(&value, source),
                _ => false,
            };

            if is_useless {
                let loc = op_assign.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(source, line, column, MSG.to_string()));
            }
        }
    }
}

fn is_zero(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    if let Some(int_node) = node.as_integer_node() {
        let src = &source.as_bytes()
            [int_node.location().start_offset()..int_node.location().end_offset()];
        return src == b"0";
    }
    if let Some(float_node) = node.as_float_node() {
        let src = &source.as_bytes()
            [float_node.location().start_offset()..float_node.location().end_offset()];
        return src == b"0.0";
    }
    false
}

fn is_one(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    if let Some(int_node) = node.as_integer_node() {
        let src = &source.as_bytes()
            [int_node.location().start_offset()..int_node.location().end_offset()];
        return src == b"1";
    }
    if let Some(float_node) = node.as_float_node() {
        let src = &source.as_bytes()
            [float_node.location().start_offset()..float_node.location().end_offset()];
        return src == b"1.0";
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UselessNumericOperation,
        "cops/lint/useless_numeric_operation"
    );
}
