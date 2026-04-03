use crate::cop::shared::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Performance/Caller — flags `caller.first`, `caller[n]`, `caller_locations.first`,
/// `caller_locations[n]` and suggests `caller(n..n).first` instead.
///
/// ## Investigation (2026-03-20)
/// FP=5 in extended corpus from two patterns:
/// 1. `caller_locations&.first` — safe navigation `&.` not matched by RuboCop (uses `send` not `csend`)
/// 2. `caller.first(n)` / `caller_locations.first(n)` — `first` with arguments returns an array,
///    not a single element. RuboCop's pattern `(send #slow_caller? :first)` only matches
///    zero-argument `first`.
///
/// Fixed by checking `call_operator_loc()` for `&.` and rejecting `first` with arguments.
pub struct Caller;

impl Cop for Caller {
    fn name(&self) -> &'static str {
        "Performance/Caller"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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
        // Pattern: caller.first, caller[n], caller_locations.first, caller_locations[n]
        // Also: caller(n).first, caller(n)[n]
        if let Some(chain) = as_method_chain(node) {
            let is_caller = chain.inner_method == b"caller";
            let is_caller_locations = chain.inner_method == b"caller_locations";
            if (is_caller || is_caller_locations) && chain.inner_call.receiver().is_none() {
                // inner call must have 0 or 1 integer arguments:
                //   caller.first / caller[n]       — 0 args, flagged
                //   caller(1).first / caller(2)[1] — 1 integer arg, flagged
                //   caller(1, 1).first             — 2 args, skip
                //   caller(1..1).first             — 1 range arg (already correct form), skip
                let inner_args = chain.inner_call.arguments();
                let inner_arg_count = inner_args.as_ref().map_or(0, |a| a.arguments().len());
                if inner_arg_count > 1 {
                    return;
                }
                if inner_arg_count == 1 {
                    let arg = inner_args.unwrap().arguments().iter().next().unwrap();
                    if arg.as_integer_node().is_none() {
                        // Non-integer argument (e.g. range) — already the recommended form
                        return;
                    }
                }

                let outer_call = node.as_call_node().unwrap();

                // Skip safe navigation: caller_locations&.first is not flagged by RuboCop
                if outer_call
                    .call_operator_loc()
                    .is_some_and(|loc| loc.as_slice() == b"&.")
                {
                    return;
                }

                let is_first = chain.outer_method == b"first";
                let is_bracket = chain.outer_method == b"[]";

                if is_first {
                    // caller.first — flagged, but caller.first(n) returns an array — skip
                    let has_args = outer_call
                        .arguments()
                        .is_some_and(|args| !args.arguments().is_empty());
                    if has_args {
                        return;
                    }
                } else if is_bracket {
                    // caller[n] — only flag when the argument is a single integer
                    // caller[0..10], caller[2..-1], caller[2, 10] should NOT be flagged
                    let is_single_integer = outer_call.arguments().is_some_and(|args| {
                        let a = args.arguments();
                        a.len() == 1 && a.iter().next().unwrap().as_integer_node().is_some()
                    });
                    if !is_single_integer {
                        return;
                    }
                } else {
                    return;
                }

                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                let method = if is_caller {
                    "caller"
                } else {
                    "caller_locations"
                };
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!(
                        "Use `{method}(n..n).first` instead of `{method}.first` or `{method}[n]`."
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Caller, "cops/performance/caller");
}
