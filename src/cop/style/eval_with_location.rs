use crate::cop::shared::node_type::{
    CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE,
    INTERPOLATED_X_STRING_NODE, STRING_NODE, X_STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/EvalWithLocation
///
/// Checks that `eval`, `class_eval`, `module_eval`, and `instance_eval` are called
/// with `__FILE__` and `__LINE__` arguments for proper error backtraces.
///
/// Also validates that the `__LINE__` offset is correct: heredoc arguments should
/// use `__LINE__ + 1` (since the heredoc body starts on the next line), while
/// inline string arguments should use `__LINE__`.
///
/// Fixed issues:
/// - FP: `eval()` with no arguments (no string literal) was incorrectly flagged.
/// - FN: Calls with correct argument count but incorrect `__LINE__` offset
///   (e.g., `__LINE__` instead of `__LINE__ + 1` for heredocs, or literal integers
///   instead of `__LINE__`) were not detected.
/// - FN: String-literal eval calls with attached blocks (for example
///   `eval "code" do ... end`) were skipped entirely by an unconditional
///   block check, even though RuboCop still requires location arguments there.
///   Block-only forms like `class_eval do ... end` remain ignored because the
///   first argument is not a string literal.
/// - FN: Once a call had enough positional arguments, the cop only validated
///   the line argument and never checked that the file argument was actually
///   `__FILE__`, so cases like `module_eval(..., loc[:file], loc[:line])`
///   were missed.
/// - FP: Backtick and `%x[...]` command strings were treated like regular
///   string literals, but RuboCop only checks plain/interpolated strings for
///   this cop. Excluding `xstr` avoids flagging `eval \`...\`` forms that
///   RuboCop accepts.
/// - FP/FN: Incorrect-line and incorrect-file offenses were reported at the
///   call node start, but RuboCop reports them at the specific argument node.
///   For multi-line calls (e.g., `class_eval %{ ... }, __FILE__, __LINE__`)
///   this caused both a FP at the call start line and a FN at the argument
///   line. Fixed by reporting at `line_arg.location()` / `file_arg.location()`.
/// - FN: Parenthesized `(__LINE__ + 1)` was not recognized by
///   `should_check_line_arg` because `ParenthesesNode` didn't match any of
///   the checked node types. Added explicit handling so the cop flags the
///   redundant parentheses (matching RuboCop's `line_with_offset?` behavior).
pub struct EvalWithLocation;

const EVAL_METHODS: &[&[u8]] = &[b"eval", b"class_eval", b"module_eval", b"instance_eval"];

impl EvalWithLocation {
    fn is_eval_method(name: &[u8]) -> bool {
        EVAL_METHODS.contains(&name)
    }

    fn requires_binding(name: &[u8]) -> bool {
        name == b"eval"
    }

    fn is_string_arg(node: &ruby_prism::Node<'_>) -> bool {
        node.as_string_node().is_some() || node.as_interpolated_string_node().is_some()
    }

    /// Check if a string node is a heredoc (opening starts with `<<`).
    fn is_heredoc(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(s) = node.as_string_node() {
            return s
                .opening_loc()
                .is_some_and(|o| o.as_slice().starts_with(b"<<"));
        }
        if let Some(s) = node.as_interpolated_string_node() {
            return s
                .opening_loc()
                .is_some_and(|o| o.as_slice().starts_with(b"<<"));
        }
        false
    }

    fn is_file_arg(node: &ruby_prism::Node<'_>) -> bool {
        node.as_source_file_node().is_some() || node.location().as_slice() == b"__FILE__"
    }

    /// Determine whether the line argument should be validated.
    /// Returns false for variables and non-arithmetic method calls (matching RuboCop behavior).
    fn should_check_line_arg(node: &ruby_prism::Node<'_>) -> bool {
        // __LINE__ keyword
        if node.as_source_line_node().is_some() {
            return true;
        }
        // Integer literal (e.g., 123)
        if node.as_integer_node().is_some() {
            return true;
        }
        // String literal (e.g., 'bar')
        if node.as_string_node().is_some() {
            return true;
        }
        // Call node: only check +/- (arithmetic on __LINE__)
        if let Some(call) = node.as_call_node() {
            let method = call.name().as_slice();
            if method == b"+" || method == b"-" {
                return true;
            }
        }
        // Parenthesized expression: e.g., (__LINE__ + 1)
        // RuboCop checks these — the parens make it not match `line_with_offset?`
        if node.as_parentheses_node().is_some() {
            return true;
        }
        // Variables, other method calls → skip
        false
    }

    /// Extract the __LINE__ offset from a line argument node.
    /// Returns Some(offset) for __LINE__-based expressions, None for literals/unknowns.
    fn get_line_offset(node: &ruby_prism::Node<'_>) -> Option<i64> {
        // __LINE__ → offset 0
        if node.as_source_line_node().is_some() {
            return Some(0);
        }
        // __LINE__ + N or __LINE__ - N
        if let Some(call) = node.as_call_node() {
            let method = call.name().as_slice();
            let is_plus = method == b"+";
            let is_minus = method == b"-";
            if !is_plus && !is_minus {
                return None;
            }
            if let Some(recv) = call.receiver() {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 {
                        // Pattern: __LINE__ + N or __LINE__ - N
                        if recv.as_source_line_node().is_some() {
                            if let Some(int_node) = arg_list[0].as_integer_node() {
                                let src = int_node.location().as_slice();
                                if let Ok(s) = std::str::from_utf8(src) {
                                    if let Ok(n) = s.parse::<i64>() {
                                        return Some(if is_plus { n } else { -n });
                                    }
                                }
                            }
                        }
                        // Pattern: N + __LINE__
                        if is_plus && arg_list[0].as_source_line_node().is_some() {
                            if let Some(int_node) = recv.as_integer_node() {
                                let src = int_node.location().as_slice();
                                if let Ok(s) = std::str::from_utf8(src) {
                                    if let Ok(n) = s.parse::<i64>() {
                                        return Some(n);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Literal integer, string, or unknown → not __LINE__ based
        None
    }

    /// Format the expected line expression for the error message.
    fn format_expected_line(offset: i64) -> String {
        if offset == 0 {
            "`__LINE__`".to_string()
        } else if offset > 0 {
            format!("`__LINE__ + {}`", offset)
        } else {
            format!("`__LINE__ - {}`", -offset)
        }
    }

    /// Get the source text of a node, wrapped in backticks for the error message.
    fn get_source_text(node: &ruby_prism::Node<'_>) -> String {
        let loc = node.location();
        let src = std::str::from_utf8(loc.as_slice()).unwrap_or("?");
        format!("`{}`", src)
    }
}

impl Cop for EvalWithLocation {
    fn name(&self) -> &'static str {
        "Style/EvalWithLocation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
            INTERPOLATED_STRING_NODE,
            INTERPOLATED_X_STRING_NODE,
            STRING_NODE,
            X_STRING_NODE,
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        if !Self::is_eval_method(method_bytes) {
            return;
        }

        let receiver = call.receiver();

        // For `eval`, only allow no receiver, Kernel, or ::Kernel
        if method_bytes == b"eval" {
            if let Some(ref recv) = receiver {
                let is_kernel = recv
                    .as_constant_read_node()
                    .is_some_and(|c| c.name().as_slice() == b"Kernel");
                let is_scoped_kernel = recv.as_constant_path_node().is_some_and(|cp| {
                    cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"Kernel")
                });
                if !is_kernel && !is_scoped_kernel {
                    return;
                }
            }
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => {
                // No arguments at all — only flag if this is not bare eval()
                // RuboCop requires a string literal first arg to flag.
                return;
            }
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();

        if arg_list.is_empty() {
            return;
        }

        // First arg must be a string-like expression (code to eval)
        let first_arg = &arg_list[0];

        // If first arg is not a string/heredoc, it might be a variable - skip
        if !Self::is_string_arg(first_arg) {
            return;
        }

        let needs_binding = Self::requires_binding(method_bytes);
        let method_str = std::str::from_utf8(method_bytes).unwrap_or("eval");

        // For eval: need (code, binding, __FILE__, __LINE__)
        // For class_eval/module_eval/instance_eval: need (code, __FILE__, __LINE__)
        let expected_count = if needs_binding { 4 } else { 3 };

        if arg_list.len() < expected_count {
            // Not enough args — report missing location info
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let msg = if needs_binding {
                format!(
                    "Pass a binding, `__FILE__`, and `__LINE__` to `{}`.",
                    method_str
                )
            } else {
                format!("Pass `__FILE__` and `__LINE__` to `{}`.", method_str)
            };
            diagnostics.push(self.diagnostic(source, line, column, msg));
        } else {
            let file_arg_idx = if needs_binding { 2 } else { 1 };
            let file_arg = &arg_list[file_arg_idx];

            if !Self::is_file_arg(file_arg) {
                let loc = file_arg.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let actual_str = Self::get_source_text(file_arg);
                let msg = format!(
                    "Incorrect file for `{}`; use `__FILE__` instead of {}.",
                    method_str, actual_str
                );
                diagnostics.push(self.diagnostic(source, line, column, msg));
            }

            // Have enough args — validate that the line argument is correct
            let line_arg_idx = expected_count - 1;
            let line_arg = &arg_list[line_arg_idx];

            // Skip validation for variables and non-arithmetic method calls
            if !Self::should_check_line_arg(line_arg) {
                return;
            }

            // Compute expected line offset:
            // For heredocs, the code body starts on the next line → offset = 1
            // For inline strings, the code is on the same line → offset = 0
            let is_heredoc = Self::is_heredoc(first_arg);
            let first_content_line = if is_heredoc {
                source
                    .offset_to_line_col(first_arg.location().start_offset())
                    .0
                    + 1
            } else {
                source
                    .offset_to_line_col(first_arg.location().start_offset())
                    .0
            };
            let line_arg_line = source
                .offset_to_line_col(line_arg.location().start_offset())
                .0;
            let expected_offset: i64 = first_content_line as i64 - line_arg_line as i64;

            // Get actual offset from the line argument
            let actual_offset = Self::get_line_offset(line_arg);

            if actual_offset != Some(expected_offset) {
                let loc = line_arg.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let expected_str = Self::format_expected_line(expected_offset);
                let actual_str = Self::get_source_text(line_arg);
                let msg = format!(
                    "Incorrect line number for `{}`; use {} instead of {}.",
                    method_str, expected_str, actual_str
                );
                diagnostics.push(self.diagnostic(source, line, column, msg));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EvalWithLocation, "cops/style/eval_with_location");
}
