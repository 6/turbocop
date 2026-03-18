use crate::cop::node_type::{CALL_NODE, FLOAT_NODE, INTEGER_NODE, NIL_NODE, WHEN_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/FloatComparison: detects unreliable float equality comparisons.
///
/// ## Investigation (2026-03-17)
/// 191 FN all from the same pattern: `.to_f` method calls compared with `==`/`!=`.
/// Root cause: `is_float()` only checked for `FloatNode` literals, not for method calls
/// that return floats (`.to_f`, `.fdiv`, `Float()`). Fixed by extending `is_float()`
/// to also detect `CallNode` with float-returning method names, matching RuboCop's
/// `FLOAT_RETURNING_METHODS = [:to_f, :Float, :fdiv]`.
///
/// ## Investigation (2026-03-18)
/// 68 FN from float literals in `when` clauses of case statements.
/// Root cause: cop only handled CallNode for `==`/`!=`/`eql?`/`equal?`, missing
/// RuboCop's `on_case` handler. Fixed by adding WhenNode handling that checks each
/// condition for float literals, using the dedicated MSG_CASE message.
///
/// ## Investigation (2026-03-18, round 2)
/// 57 FN from binary operations on floats and float instance methods.
/// Root cause: `is_float()` only checked for `FloatNode` and float-returning method
/// names (to_f, fdiv, Float), missing RuboCop's recursive `float_send?` logic:
/// - Arithmetic operations (+, -, *, /, **, %) with a float operand produce floats
/// - Float instance methods (abs, magnitude, next_float, prev_float, etc.) on float receivers
/// - Numeric-returning methods (ceil, floor, round, truncate) with positive precision arg
///
/// FN concentrated in jruby (20) and natalie (18) spec files comparing float constants
/// and expressions like `2.0 ** -52`, `1.0 + Float::EPSILON`, `0.0.next_float`.
///
/// ## Investigation (2026-03-18, round 3)
/// 5 FP from ParenthesesNode unwrapping. RuboCop's `float_type?` does NOT unwrap
/// parenthesized expressions — `(0.0)` is a `begin` node in RuboCop's AST and
/// `float_type?` returns false for it. Patterns like `(0.0).next_float.should == ...`
/// in jruby/natalie were falsely detected because `is_float()` recursed through
/// ParenthesesNode. Fixed by removing ParenthesesNode handling entirely.
pub struct FloatComparison;

impl Cop for FloatComparison {
    fn name(&self) -> &'static str {
        "Lint/FloatComparison"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FLOAT_NODE, INTEGER_NODE, NIL_NODE, WHEN_NODE]
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
        // Handle float literals in when clauses
        if let Some(when_node) = node.as_when_node() {
            for condition in when_node.conditions().iter() {
                if is_float(&condition) && !is_literal_safe(&condition) {
                    let loc = condition.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Avoid float literal comparisons in case statements as they are unreliable.".to_string(),
                    ));
                }
            }
            return;
        }

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();
        let is_equality = matches!(method, b"==" | b"!=" | b"eql?" | b"equal?");
        if !is_equality {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 1 {
            return;
        }

        let first_arg = args.iter().next().unwrap();

        // Skip safe comparisons: comparing to 0.0 or nil
        if is_literal_safe(&receiver) || is_literal_safe(&first_arg) {
            return;
        }

        if is_float(&receiver) || is_float(&first_arg) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let msg = if method == b"!=" {
                "Avoid inequality comparisons of floats as they are unreliable."
            } else {
                "Avoid equality comparisons of floats as they are unreliable."
            };
            diagnostics.push(self.diagnostic(source, line, column, msg.to_string()));
        }
    }
}

/// Arithmetic operators that propagate float type from either operand.
const ARITHMETIC_METHODS: &[&[u8]] = &[b"+", b"-", b"*", b"/", b"**", b"%"];

/// Float instance methods that return a float when called on a float receiver.
/// Matches RuboCop's `FLOAT_INSTANCE_METHODS`.
const FLOAT_INSTANCE_METHODS: &[&[u8]] = &[
    b"-@",
    b"abs",
    b"magnitude",
    b"modulo",
    b"next_float",
    b"prev_float",
    b"quo",
];

fn is_float(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_float_node().is_some() {
        return true;
    }

    if let Some(call) = node.as_call_node() {
        return is_float_call(call);
    }
    false
}

fn is_float_call(call: ruby_prism::CallNode<'_>) -> bool {
    let method = call.name().as_slice();

    // Float-returning methods: .to_f, .fdiv, Float()
    if matches!(method, b"to_f" | b"fdiv" | b"Float") {
        return true;
    }

    // Arithmetic operations: if either operand is float, result is float
    if ARITHMETIC_METHODS.contains(&method) {
        if let Some(receiver) = call.receiver() {
            if is_float(&receiver) {
                return true;
            }
        }
        if let Some(args) = call.arguments() {
            if let Some(first_arg) = args.arguments().iter().next() {
                if is_float(&first_arg) {
                    return true;
                }
            }
        }
        return false;
    }

    // Float instance methods on a float receiver
    if let Some(receiver) = call.receiver() {
        if is_float(&receiver) {
            // Methods that always return float from a float receiver
            if FLOAT_INSTANCE_METHODS.contains(&method) {
                return true;
            }
            // Numeric-returning methods: ceil/floor/round/truncate return float
            // only when called with a positive integer precision argument
            if matches!(method, b"ceil" | b"floor" | b"round" | b"truncate") {
                if let Some(args) = call.arguments() {
                    if let Some(first_arg) = args.arguments().iter().next() {
                        if let Some(int_node) = first_arg.as_integer_node() {
                            let src = int_node.location().as_slice();
                            // Positive integer precision -> returns float
                            if !src.starts_with(b"-") && src != b"0" {
                                return true;
                            }
                        }
                    }
                }
                // No args or non-positive precision -> returns integer
                return false;
            }
        }
    }

    false
}

fn is_literal_safe(node: &ruby_prism::Node<'_>) -> bool {
    // Comparing to 0.0 is safe
    if let Some(f) = node.as_float_node() {
        let src = f.location().as_slice();
        if src == b"0.0" || src == b"-0.0" {
            return true;
        }
    }
    // Comparing to integer 0 is safe
    if let Some(i) = node.as_integer_node() {
        let src = i.location().as_slice();
        if src == b"0" {
            return true;
        }
    }
    // Comparing to nil is safe
    if node.as_nil_node().is_some() {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FloatComparison, "cops/lint/float_comparison");
}
