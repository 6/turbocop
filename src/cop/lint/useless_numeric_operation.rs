use crate::cop::shared::node_type::{CALL_NODE, INTEGER_NODE, LOCAL_VARIABLE_OPERATOR_WRITE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation: 13 FP, 0 FN.
/// Two root causes:
/// 1. Float operands (`*= 1.0`, `+= 0.0`): RuboCop's NodePattern only matches
///    `(int $_)`, not `(float $_)`. Multiplying by 1.0 is intentional float coercion.
/// 2. Instance/class/global variable op-assigns (`@index += 0`, `@@count -= 0`,
///    `$counter += 0`): RuboCop's `useless_abbreviated_assignment?` pattern uses
///    `(op-asgn (lvasgn $_) ...)` which only matches local variable assignments.
///
/// Fix: removed float literal checks from `is_zero`/`is_one`, and removed
/// ivar/cvar/gvar operator write node handling.
///
/// Round 2: 3 FP in jruby, natalie, jetpants repos.
/// Root cause: `is_bare_method_call` didn't check that the receiver call has
/// no arguments or block. RuboCop's `(call nil? $_)` only matches bare method
/// calls without arguments (e.g., `x`), not `Complex(1, 2)` or `foo(arg)`.
/// Fix: added `arguments().is_none() && block().is_none()` to the receiver check.
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
        &[CALL_NODE, INTEGER_NODE, LOCAL_VARIABLE_OPERATOR_WRITE_NODE]
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
                Some(recv_call) => {
                    recv_call.receiver().is_none()
                        && recv_call.arguments().is_none()
                        && recv_call.block().is_none()
                }
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
    }
}

fn is_zero(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    if let Some(int_node) = node.as_integer_node() {
        let src = &source.as_bytes()
            [int_node.location().start_offset()..int_node.location().end_offset()];
        return src == b"0";
    }
    false
}

fn is_one(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    if let Some(int_node) = node.as_integer_node() {
        let src = &source.as_bytes()
            [int_node.location().start_offset()..int_node.location().end_offset()];
        return src == b"1";
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
