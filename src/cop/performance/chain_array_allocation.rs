use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ChainArrayAllocation;

/// Methods that ALWAYS return a new array.
const ALWAYS_RETURNS_NEW_ARRAY: &[&[u8]] = &[
    b"collect",
    b"compact",
    b"drop",
    b"drop_while",
    b"flatten",
    b"map",
    b"reject",
    b"reverse",
    b"rotate",
    b"select",
    b"shuffle",
    b"sort",
    b"take",
    b"take_while",
    b"transpose",
    b"uniq",
    b"values_at",
];

/// Methods that return a new array only when called with an argument.
const RETURN_NEW_ARRAY_WHEN_ARGS: &[&[u8]] = &[b"first", b"last", b"pop", b"sample", b"shift"];

/// Methods that return a new array only when called WITHOUT a block.
const RETURNS_NEW_ARRAY_WHEN_NO_BLOCK: &[&[u8]] = &[b"zip", b"product"];

/// Methods that have a mutation alternative (e.g., collect → collect!).
const HAS_MUTATION_ALTERNATIVE: &[&[u8]] = &[
    b"collect", b"compact", b"flatten", b"map", b"reject", b"reverse", b"rotate", b"select",
    b"shuffle", b"sort", b"uniq",
];

/// Check if any call in the receiver chain is `lazy`.
fn chain_contains_lazy(node: &ruby_prism::Node<'_>) -> bool {
    let mut current = node.as_call_node();
    while let Some(call) = current {
        if call.name().as_slice() == b"lazy" {
            return true;
        }
        current = call.receiver().and_then(|r| r.as_call_node());
    }
    false
}

/// Check if the inner call returns a new array based on RuboCop's rules.
fn inner_returns_new_array(inner: &ruby_prism::CallNode<'_>) -> bool {
    let name = inner.name().as_slice();

    // ALWAYS_RETURNS_NEW_ARRAY — always qualifies
    if ALWAYS_RETURNS_NEW_ARRAY.contains(&name) {
        return true;
    }

    // RETURN_NEW_ARRAY_WHEN_ARGS — only when called with an argument
    if RETURN_NEW_ARRAY_WHEN_ARGS.contains(&name) {
        return inner.arguments().is_some();
    }

    // RETURNS_NEW_ARRAY_WHEN_NO_BLOCK — only when called WITHOUT a block
    if RETURNS_NEW_ARRAY_WHEN_NO_BLOCK.contains(&name) {
        return inner.block().is_none();
    }

    false
}

impl Cop for ChainArrayAllocation {
    fn name(&self) -> &'static str {
        "Performance/ChainArrayAllocation"
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
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        // Outer method must have a mutation alternative
        if !HAS_MUTATION_ALTERNATIVE.contains(&chain.outer_method) {
            return;
        }

        // Inner method must return a new array
        if !inner_returns_new_array(&chain.inner_call) {
            return;
        }

        // Skip if `lazy` appears anywhere in the chain
        if chain_contains_lazy(node) {
            return;
        }

        // Special handling for `select` as the outer method:
        // RuboCop only flags `select` when the receiver is a block with no positional args
        // (to avoid flagging Rails' QueryMethods#select which takes positional args).
        if chain.outer_method == b"select" {
            // The receiver must be a block call (e.g., `model.select { ... }.select { ... }`)
            // and the inner call must have no positional arguments.
            let has_block = chain.inner_call.block().is_some();
            let has_args = chain.inner_call.arguments().is_some();
            if !has_block || has_args {
                return;
            }
        }

        let inner_name = String::from_utf8_lossy(chain.inner_method);
        let outer_name = String::from_utf8_lossy(chain.outer_method);

        // Point diagnostic at the outer method name (RuboCop uses node.loc.selector)
        let outer_call = node.as_call_node().unwrap();
        let loc = outer_call.message_loc().unwrap_or(node.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());

        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use unchained `{}` and `{}!` (followed by `return array` if required) instead of chaining `{}...{}`.",
                inner_name, outer_name, inner_name, outer_name
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ChainArrayAllocation,
        "cops/performance/chain_array_allocation"
    );
}
