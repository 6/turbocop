use crate::cop::shared::node_type::{CALL_NODE, OR_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=3, FN=8.
///
/// FP=3 / FN=8: the cop logic was broadly correct, but the offense location was
/// too wide. RuboCop anchors at the operator-plus-RHS range, not the start of
/// the whole `or` expression, so multiline cases produced line mismatches.
/// We also missed chained forms like `foo || x.to_s || fallback`, where the
/// truthy method call is the RHS of an inner `or` and the offense belongs on
/// the outer `or`.
///
/// Local rerun after the fix improved the cop from the CI nitrocop baseline of
/// 115 offenses to 117, leaving 3 missing offenses and no excess over the CI
/// baseline. The remaining FN were not reduced further in this phase.
///
/// ## Phase 2 fix (2026-03-19)
///
/// FN=3 remaining: all involved bare method calls (no receiver) to always-truthy
/// methods like `to_s` and `to_i`. The `is_truthy_method_call()` guard required
/// `call.receiver().is_some()`, which excluded implicit-self calls like `to_s`.
/// Removed the receiver check. Also added RHS nested-truthy detection for
/// parenthesized expressions like `x || (default || to_s)` where the inner
/// `to_s` makes the parenthesized RHS always truthy.
///
/// ## Phase 3 fix (2026-03-20)
///
/// FP=1 remaining: `@tag || (msg[0].to_s)` was incorrectly flagged. The RHS
/// parenthesized-truthy check (added in Phase 2) was overly broad — it reported
/// an offense whenever the RHS of `||` was a parenthesized expression containing
/// a truthy method call. But a plain parenthesized truthy call like `(x.to_s)`
/// is NOT an offense — only when the parens contain an inner `||` whose RHS is
/// always truthy (e.g., `a || (default || x.to_s)`). Narrowed the check to
/// `rhs_parenthesized_or_truthy` which requires an inner `||` node.
///
/// ## Phase 4 fix (2026-03-23)
///
/// FN=1 remaining: `object.get_option('logo') || (default || h.asset_url(...).to_s)`
/// from owen2345/camaleon-cms. The `rhs_parenthesized_or_truthy` function already
/// handled this pattern correctly — the inner `||` has `.to_s` as its RHS, making
/// the parenthesized expression always truthy. Added test coverage for this pattern
/// (`a || (default_val || x.to_s)`) to prevent regression.
///
/// Checks for useless OR expressions where the left side always returns a truthy value.
pub struct UselessOr;

const TRUTHY_METHODS: &[&[u8]] = &[
    b"to_a",
    b"to_c",
    b"to_d",
    b"to_i",
    b"to_f",
    b"to_h",
    b"to_r",
    b"to_s",
    b"to_sym",
    b"intern",
    b"inspect",
    b"hash",
    b"object_id",
    b"__id__",
];

impl Cop for UselessOr {
    fn name(&self) -> &'static str {
        "Lint/UselessOr"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, OR_NODE]
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
        let or_node = match node.as_or_node() {
            Some(n) => n,
            None => return,
        };

        let lhs = or_node.left();
        if is_truthy_method_call(&lhs) {
            report_offense(self, source, &or_node, &lhs, diagnostics);
            return;
        }

        if let Some(truthy_node) = nested_truthy_middle(&lhs) {
            report_offense(self, source, &or_node, &truthy_node, diagnostics);
            return;
        }

        // RuboCop also flags the outer `||` when its RHS is a parenthesized `||`
        // whose inner RHS is always truthy. E.g.:
        //   a || (default || x.to_s)
        // The inner `x.to_s` makes the parenthesized expression always truthy,
        // so the outer `||`'s RHS is always truthy. Note: this is different from
        // a plain parenthesized truthy call (e.g. `a || (x.to_s)`) which is NOT
        // an offense — only when the parens contain an `||` with a truthy RHS.
        if let Some(truthy_node) = rhs_parenthesized_or_truthy(&or_node.right()) {
            report_offense(self, source, &or_node, &truthy_node, diagnostics);
        }
    }
}

fn is_truthy_method_call(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    // Must have no arguments (explicit empty parentheses are allowed).
    if call
        .arguments()
        .is_some_and(|arguments| !arguments.arguments().is_empty())
    {
        return false;
    }

    // Must not be safe navigation (&.) - safe navigation can return nil
    if let Some(op) = call.call_operator_loc() {
        if op.as_slice() == b"&." {
            return false;
        }
    }

    let method_name = call.name().as_slice();
    TRUTHY_METHODS.contains(&method_name)
}

fn nested_truthy_middle<'pr>(node: &ruby_prism::Node<'pr>) -> Option<ruby_prism::Node<'pr>> {
    if let Some(or_node) = node.as_or_node() {
        let rhs = or_node.right();
        if is_truthy_method_call(&rhs) {
            return Some(rhs);
        }
        return nested_truthy_middle(&rhs).or_else(|| nested_truthy_middle(&or_node.left()));
    }

    if let Some(parens) = node.as_parentheses_node() {
        if let Some(body) = parens.body() {
            if let Some(stmts) = body.as_statements_node() {
                for stmt in stmts.body().iter() {
                    if let Some(truthy_node) = nested_truthy_middle(&stmt) {
                        return Some(truthy_node);
                    }
                }
            }
        }
    }

    None
}

/// Check if a node is a parenthesized `||` expression whose RHS is always truthy.
/// E.g. `(default || x.to_s)` — the inner `x.to_s` makes the whole expression
/// always truthy. Returns the truthy node if found.
///
/// This does NOT flag plain parenthesized truthy calls like `(x.to_s)` — only
/// when the parens contain an `||` with a truthy method call as its RHS.
fn rhs_parenthesized_or_truthy<'pr>(node: &ruby_prism::Node<'pr>) -> Option<ruby_prism::Node<'pr>> {
    let parens = node.as_parentheses_node()?;
    let body = parens.body()?;
    let stmts = body.as_statements_node()?;
    let mut iter = stmts.body().iter();
    let stmt = iter.next()?;
    // Only handle single-statement parentheses
    if iter.next().is_some() {
        return None;
    }
    let inner_or = stmt.as_or_node()?;
    let rhs = inner_or.right();
    if is_truthy_method_call(&rhs) {
        return Some(rhs);
    }
    // Also handle deeper nesting: (a || (b || x.to_s))
    rhs_parenthesized_or_truthy(&rhs)
}

fn report_offense(
    cop: &UselessOr,
    source: &SourceFile,
    or_node: &ruby_prism::OrNode<'_>,
    truthy_node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let lhs_src = node_source(source, truthy_node);
    let rhs_src = node_source(source, &or_node.right());
    let op_loc = or_node.operator_loc();
    let (line, column) = source.offset_to_line_col(op_loc.start_offset());
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!(
            "`{}` will never evaluate because `{}` always returns a truthy value.",
            rhs_src, lhs_src
        ),
    ));
}

fn node_source<'a>(source: &'a SourceFile, node: &ruby_prism::Node<'_>) -> &'a str {
    let loc = node.location();
    source.byte_slice(loc.start_offset(), loc.end_offset(), "...")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessOr, "cops/lint/useless_or");
}
