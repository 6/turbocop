use crate::cop::shared::node_type::{CALL_NODE, LOCAL_VARIABLE_OPERATOR_WRITE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for numeric operations that have a constant result.
/// For example: `x * 0` always returns 0, `x / x` always returns 1.
///
/// ## Corpus investigation (2026-03-08)
/// FP=39 from jruby/natalie: `array * 0` (Array#*), `complex ** 0`, `rational ** 0`.
/// Root cause: cop was flagging any `x * 0` regardless of receiver type.
/// RuboCop restricts to `(call nil? $_lhs)` — only bare method calls (identifiers
/// not assigned as local variables). Local variable reads like `array = []; array * 0`
/// are excluded because the type is unknown (could be Array, String, etc.).
/// Fix: only flag when receiver is a CallNode with no receiver (bare method call),
/// not LocalVariableReadNode or any other expression type.
///
/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=0, FN=2.
///
/// FN fix:
/// - Prism represents `x *= 0` as `LocalVariableOperatorWriteNode`, not a
///   `CallNode`. The initial implementation only checked plain operator calls
///   (`x * 0`, `x / x`, `x ** 0`) and skipped abbreviated assignment forms.
pub struct NumericOperationWithConstantResult;

impl Cop for NumericOperationWithConstantResult {
    fn name(&self) -> &'static str {
        "Lint/NumericOperationWithConstantResult"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, LOCAL_VARIABLE_OPERATOR_WRITE_NODE]
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
        if let Some(op_assign) = node.as_local_variable_operator_write_node() {
            let operator = op_assign.binary_operator().as_slice();
            let value = op_assign.value();

            let has_constant_result = match operator {
                b"*" => is_zero(&value, source),
                b"**" => is_zero(&value, source),
                _ => false,
            };

            if has_constant_result {
                let loc = op_assign.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Numeric operation with a constant result detected.".to_string(),
                ));
            }
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Only check *, /, **
        if method_name != b"*" && method_name != b"/" && method_name != b"**" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // RuboCop restricts to `(call nil? $_lhs)` — only bare method calls
        // (identifiers that haven't been assigned as local variables).
        // This avoids false positives on Array#*, String#*, etc.
        if !is_bare_method_call(&receiver) {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args: Vec<_> = arguments.arguments().iter().collect();
        if args.len() != 1 {
            return;
        }

        let rhs = &args[0];

        let has_constant_result = if is_zero(rhs, source) {
            // x * 0 => 0, x ** 0 => 1
            method_name == b"*" || method_name == b"**"
        } else if method_name == b"/" && is_same_bare_method(&receiver, rhs, source) {
            // x / x => 1 (both must be bare method calls with same name)
            true
        } else {
            false
        };

        if !has_constant_result {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Numeric operation with a constant result detected.".to_string(),
        ));
    }
}

/// Returns true if the node is a bare method call (no receiver, no arguments).
/// In RuboCop's Parser AST, this is `(send nil :name)` — an identifier that
/// hasn't been assigned as a local variable. In Prism, it's a `CallNode` with
/// no receiver and no arguments.
fn is_bare_method_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        return call.receiver().is_none() && call.arguments().is_none();
    }
    false
}

fn is_zero(node: &ruby_prism::Node<'_>, source: &SourceFile) -> bool {
    if let Some(int_node) = node.as_integer_node() {
        let src = &source.as_bytes()
            [int_node.location().start_offset()..int_node.location().end_offset()];
        return src == b"0";
    }
    false
}

/// For `x / x`, both sides must be bare method calls with the same name.
/// RuboCop pattern: `(call (call nil? $_lhs) :/ (call nil? $_rhs))` with lhs == rhs.
fn is_same_bare_method(
    a: &ruby_prism::Node<'_>,
    b: &ruby_prism::Node<'_>,
    source: &SourceFile,
) -> bool {
    if !is_bare_method_call(b) {
        return false;
    }
    let a_src = &source.as_bytes()[a.location().start_offset()..a.location().end_offset()];
    let b_src = &source.as_bytes()[b.location().start_offset()..b.location().end_offset()];
    a_src == b_src
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NumericOperationWithConstantResult,
        "cops/lint/numeric_operation_with_constant_result"
    );
}
