use crate::cop::shared::node_type::{
    CALL_NODE, EMBEDDED_STATEMENTS_NODE, INTERPOLATED_REGULAR_EXPRESSION_NODE,
    INTERPOLATED_STRING_NODE, INTERPOLATED_SYMBOL_NODE, INTERPOLATED_X_STRING_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for redundant `.to_s` in string interpolation and in arguments to
/// `puts`, `print`, and `warn` (bare calls only, no receiver).
///
/// ## Investigation notes
/// - RuboCop's `RESTRICT_ON_SEND = %i[print puts warn]` — `p` is NOT included
///   because `p` calls `.inspect`, not `.to_s`.
/// - RuboCop's `on_send` checks `node.receiver` is nil, so `$stdout.puts x.to_s`
///   and `obj.print x.to_s` are NOT flagged.
/// - The FN=103 from the corpus were all `puts x.to_s` patterns in bare calls.
///
/// ## FN fixes (2026-03)
/// Three root causes for FN=50:
/// 1. Bare `.to_s` (implicit receiver / no receiver) in interpolation and print
///    calls was not detected — `to_s` without a receiver is a CallNode with
///    `receiver().is_none()`. RuboCop uses a different message: "Use `self`
///    instead of `Object#to_s`".
/// 2. Interpolation in regex (`/#{x.to_s}/`), backtick xstring (`` `#{x.to_s}` ``),
///    and symbol (`:"#{x.to_s}"`) was not checked — only InterpolatedStringNode
///    was handled. Added InterpolatedRegularExpressionNode, InterpolatedXStringNode,
///    and InterpolatedSymbolNode.
/// 3. Multi-expression interpolation `#{top; result.to_s}` — RuboCop checks the
///    *last* statement, not requiring exactly one statement. Fixed by checking
///    body.last() instead of requiring body.len() == 1.
pub struct RedundantStringCoercion;

const PRINT_METHODS: &[&[u8]] = &[b"puts", b"print", b"warn"];

impl Cop for RedundantStringCoercion {
    fn name(&self) -> &'static str {
        "Lint/RedundantStringCoercion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            EMBEDDED_STATEMENTS_NODE,
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
            INTERPOLATED_STRING_NODE,
            INTERPOLATED_SYMBOL_NODE,
            INTERPOLATED_X_STRING_NODE,
        ]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(interp) = node.as_interpolated_string_node() {
            self.check_interpolation(source, &interp.parts(), diagnostics, &mut corrections);
            return;
        }

        if let Some(interp) = node.as_interpolated_regular_expression_node() {
            self.check_interpolation(source, &interp.parts(), diagnostics, &mut corrections);
            return;
        }

        if let Some(interp) = node.as_interpolated_x_string_node() {
            self.check_interpolation(source, &interp.parts(), diagnostics, &mut corrections);
            return;
        }

        if let Some(interp) = node.as_interpolated_symbol_node() {
            self.check_interpolation(source, &interp.parts(), diagnostics, &mut corrections);
            return;
        }

        if let Some(call) = node.as_call_node() {
            self.check_print_call(source, call, diagnostics, &mut corrections);
        }
    }
}

impl RedundantStringCoercion {
    fn check_interpolation(
        &self,
        source: &SourceFile,
        parts: &ruby_prism::NodeList<'_>,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    ) {
        for part in parts {
            let embedded = match part.as_embedded_statements_node() {
                Some(e) => e,
                None => continue,
            };

            let statements = match embedded.statements() {
                Some(s) => s,
                None => continue,
            };

            let body = statements.body();
            if body.is_empty() {
                continue;
            }

            // RuboCop checks the last expression in multi-statement interpolation
            let last = match body.last() {
                Some(n) => n,
                None => continue,
            };

            let call = match last.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            if call.name().as_slice() != b"to_s" {
                continue;
            }

            if call.arguments().is_some() {
                continue;
            }

            let implicit_receiver = call.receiver().is_none();
            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let message = if implicit_receiver {
                "Use `self` instead of `Object#to_s` in interpolation.".to_string()
            } else {
                "Redundant use of `Object#to_s` in interpolation.".to_string()
            };
            let mut diag = self.diagnostic(source, line, column, message);
            // Autocorrect: remove `.to_s` or replace bare `to_s` with `self`
            if let Some(corr) = corrections {
                if implicit_receiver {
                    // Replace `to_s` with `self`
                    corr.push(crate::correction::Correction {
                        start: call.location().start_offset(),
                        end: call.location().end_offset(),
                        replacement: "self".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                } else {
                    // Remove `.to_s` (from the dot before `to_s` to end)
                    let receiver = call.receiver().unwrap();
                    corr.push(crate::correction::Correction {
                        start: receiver.location().end_offset(),
                        end: call.location().end_offset(),
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                }
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
    }

    fn check_print_call(
        &self,
        source: &SourceFile,
        call: ruby_prism::CallNode<'_>,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Only bare calls (no receiver) — matches RuboCop's `return if node.receiver`
        if call.receiver().is_some() {
            return;
        }

        let method_name = call.name();
        if !PRINT_METHODS.contains(&method_name.as_slice()) {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let method_name_str = std::str::from_utf8(method_name.as_slice()).unwrap_or("puts");

        for arg in &args.arguments() {
            let arg_call = match arg.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            if arg_call.name().as_slice() != b"to_s" {
                continue;
            }

            // Ensure to_s has no arguments
            if arg_call.arguments().is_some() {
                continue;
            }

            let implicit_receiver = arg_call.receiver().is_none();
            let loc = arg_call.message_loc().unwrap_or(arg_call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let message = if implicit_receiver {
                format!("Use `self` instead of `Object#to_s` in `{method_name_str}`.")
            } else {
                format!("Redundant use of `Object#to_s` in `{method_name_str}`.")
            };
            let mut diag = self.diagnostic(source, line, column, message);
            if let Some(corr) = corrections {
                if implicit_receiver {
                    corr.push(crate::correction::Correction {
                        start: arg_call.location().start_offset(),
                        end: arg_call.location().end_offset(),
                        replacement: "self".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                } else {
                    let receiver = arg_call.receiver().unwrap();
                    corr.push(crate::correction::Correction {
                        start: receiver.location().end_offset(),
                        end: arg_call.location().end_offset(),
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                }
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantStringCoercion,
        "cops/lint/redundant_string_coercion"
    );
    crate::cop_autocorrect_fixture_tests!(
        RedundantStringCoercion,
        "cops/lint/redundant_string_coercion"
    );
}
