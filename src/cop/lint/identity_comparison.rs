use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for `object_id` comparisons using `==` or `!=`, which should use `equal?` instead.
///
/// ## Investigation (2026-03-08)
///
/// Previous implementation was completely wrong: it checked `x.equal?(x)` (same source text on
/// receiver and argument of `.equal?`). RuboCop's actual behavior is to flag
/// `foo.object_id == bar.object_id` and `foo.object_id != bar.object_id` patterns, suggesting
/// `equal?` / `!equal?` instead. The old implementation had 0% match rate (FP=35 from flagging
/// `.equal?` calls in spec files, FN=30 from missing the actual `object_id` comparison pattern).
///
/// Root cause: Misunderstanding of what the cop detects. RuboCop's `Lint/IdentityComparison`
/// uses `RESTRICT_ON_SEND = %i[== !=]` and matches `(send (send _lhs :object_id) {== !=} (send _rhs :object_id))`.
/// Both sides must be `.object_id` calls with explicit receivers (bare `object_id` without
/// receiver is not flagged).
pub struct IdentityComparison;

impl Cop for IdentityComparison {
    fn name(&self) -> &'static str {
        "Lint/IdentityComparison"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Only interested in == and !=
        let is_eq = method_name == b"==";
        let is_neq = method_name == b"!=";
        if !is_eq && !is_neq {
            return;
        }

        // Receiver must be a call to .object_id with an explicit receiver
        let lhs = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        let lhs_call = match lhs.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if lhs_call.name().as_slice() != b"object_id" {
            return;
        }
        // lhs must have a receiver (not bare `object_id`)
        if lhs_call.receiver().is_none() {
            return;
        }

        // Argument must be a single call to .object_id with an explicit receiver
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list = args.arguments();
        if arg_list.len() != 1 {
            return;
        }
        let rhs = match arg_list.iter().next() {
            Some(a) => a,
            None => return,
        };
        let rhs_call = match rhs.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if rhs_call.name().as_slice() != b"object_id" {
            return;
        }
        // rhs must have a receiver (not bare `object_id`)
        if rhs_call.receiver().is_none() {
            return;
        }

        let (comparison, bang) = if is_eq { ("==", "") } else { ("!=", "!") };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `{}equal?` instead of `{}` when comparing `object_id`.",
                bang, comparison
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IdentityComparison, "cops/lint/identity_comparison");
}
