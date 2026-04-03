use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for comparison chains like `x < y < z`.
///
/// ## FP fix (2026-03): Skip set operations as center value
/// RuboCop skips flagging when the center value (RHS of the inner comparison)
/// is a set operation (`&`, `|`, `^`). Due to Ruby operator precedence,
/// `x >= y & z < w` parses as `(x >= (y & z)) < w`. The center value `(y & z)`
/// uses set operation `&`, so RuboCop does not flag it.
///
/// ## FP fix (2026-04): Require regular arguments on both comparison sends
/// RuboCop only matches chained comparisons when both comparison method sends
/// have a regular argument. In Prism, overloaded operator-method chains like
/// `Success(1).>= {|v| ... }.>= -> v { ... }` still appear as nested `CallNode`s
/// named `>=`, but the inner call's operand is a block rather than an argument.
/// Treating every nested operator call as a comparison caused false positives in
/// monadic bind-style APIs.
pub struct MultipleComparison;

impl Cop for MultipleComparison {
    fn name(&self) -> &'static str {
        "Lint/MultipleComparison"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        // Pattern: (send (send _ COMP _) COMP _)
        // i.e., x < y < z
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let outer_method = outer_call.name().as_slice();
        if !is_comparison(outer_method) {
            return;
        }

        // The receiver of the outer call should itself be a comparison call
        let receiver = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let inner_method = inner_call.name().as_slice();
        if !is_comparison(inner_method) {
            return;
        }

        // Match RuboCop's send pattern shape: both comparisons must have exactly
        // one regular argument. This excludes overloaded operator-method chains
        // whose operand is carried by a block instead of an argument.
        let inner_args = match inner_call.arguments() {
            Some(args) if args.arguments().len() == 1 => args,
            _ => return,
        };

        match outer_call.arguments() {
            Some(args) if args.arguments().len() == 1 => {}
            _ => return,
        }

        let center = match inner_args.arguments().iter().next() {
            Some(arg) => arg,
            None => return,
        };

        // Check if the center value (RHS of inner comparison) is a set operation.
        // Due to Ruby operator precedence, `x >= y & z < w` parses as
        // `(x >= (y & z)) < w`. RuboCop skips these cases.
        if let Some(center_call) = center.as_call_node() {
            let center_method = center_call.name().as_slice();
            if is_set_operation(center_method) {
                return;
            }
        }

        let loc = outer_call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use the `&&` operator to compare multiple values.".to_string(),
        ));
    }
}

fn is_comparison(method: &[u8]) -> bool {
    matches!(method, b"<" | b">" | b"<=" | b">=")
}

fn is_set_operation(method: &[u8]) -> bool {
    matches!(method, b"&" | b"|" | b"^")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultipleComparison, "cops/lint/multiple_comparison");
}
