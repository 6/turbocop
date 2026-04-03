use crate::cop::shared::method_identifier_predicates;
use crate::cop::shared::node_type::{AND_NODE, CALL_NODE, OR_NODE};
use crate::cop::shared::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-15)
///
/// Corpus oracle reported FP=0, FN=1.
///
/// FN fix:
/// - RuboCop also flags calls without parentheses when the first argument is a
///   ternary whose condition uses `&&` or `||` (for example
///   `puts ready && synced ? "ok" : "missing"`). The initial implementation
///   only handled predicate methods with boolean operator arguments and missed
///   the ternary-first-argument path entirely.
pub struct RequireParentheses;

fn is_assignment_method(call: &ruby_prism::CallNode<'_>) -> bool {
    method_identifier_predicates::is_assignment_method(call.name().as_slice())
}

impl Cop for RequireParentheses {
    fn name(&self) -> &'static str {
        "Lint/RequireParentheses"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[AND_NODE, CALL_NODE, OR_NODE]
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

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        // Must NOT have parentheses
        if call.opening_loc().is_some() {
            return;
        }

        if let Some(first_arg) = args.arguments().iter().next() {
            if let Some(ternary) = first_arg.as_if_node() {
                let condition = ternary.predicate();
                if util::is_ternary(&ternary)
                    && !is_assignment_method(&call)
                    && call.name().as_slice() != b"[]"
                    && (condition.as_and_node().is_some() || condition.as_or_node().is_some())
                {
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Use parentheses in the method call to avoid confusion about precedence."
                            .to_string(),
                    ));
                    return;
                }
            }
        }

        let name = call.name();
        if !name.as_slice().ends_with(b"?") {
            return;
        }

        let has_boolean_arg = args
            .arguments()
            .iter()
            .any(|arg| arg.as_and_node().is_some() || arg.as_or_node().is_some());

        if has_boolean_arg {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(
                self.diagnostic(
                    source,
                    line,
                    column,
                    "Use parentheses in the method call to avoid confusion about precedence."
                        .to_string(),
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RequireParentheses, "cops/lint/require_parentheses");
}
