use crate::cop::shared::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-03):
/// FP fix: Inner ALWAYS_RETURNS_NEW_ARRAY methods with both a real block AND positional args
/// (e.g., `Parallel.map(items) { }`) are custom methods, not Array#map. RuboCop's NodePattern
/// `(any_block (send _ :method) ...)` requires the inner send to have NO args when blocked.
/// FN fix: Added operator methods `*`, `+`, `-`, `|` to ALWAYS_RETURNS_NEW_ARRAY to match
/// RuboCop's list (these return new arrays and the chained mutation can be used instead).
///
/// Corpus investigation (2026-03-04):
/// FN fix: Block pass (`&method(:name)`, `&:sym`) was incorrectly treated as a real block,
/// causing `Parallel.map(items, &method(:name)).flatten` to be skipped. In Parser gem's AST,
/// `block_pass` is part of the send's arguments (matching pattern 3), not a block wrapper
/// (pattern 2). Fixed by checking `as_block_node()` instead of `block().is_some()` to
/// distinguish real blocks from block arguments. Also confirmed `.flatten(1)` with depth arg
/// works correctly (same root cause as the block_pass issue in corpus cases).
///
/// ## Extended corpus investigation (2026-03-23)
///
/// Extended corpus reported FP=0, FN=1. The single FN is from
/// brixen__poetics__b382a80 at `bin/poetics:88` (`@history.last(100).map { |s| f.puts s }`).
/// The cop logic correctly detects this pattern (verified by unit test at
/// offense.rb:38). The FN is a file-discovery asymmetry: `bin/poetics` is an
/// extensionless Ruby script. Nitrocop detects extensionless Ruby files via
/// shebang detection (`has_ruby_shebang`). The baseline config also excludes
/// `bin/**/*` via AllCops.Exclude, which should drop this file from both tools'
/// comparison once the bin/**/* exclude pattern is properly applied.
/// If the FN persists, the file may lack a Ruby shebang or RuboCop may
/// discover it through a different mechanism (e.g., git ls-files).
pub struct ChainArrayAllocation;

/// Methods that ALWAYS return a new array.
const ALWAYS_RETURNS_NEW_ARRAY: &[&[u8]] = &[
    b"*",
    b"+",
    b"-",
    b"|",
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

/// Check if an argument node matches RuboCop's NodePattern `{int lvar ivar cvar gvar send}`.
/// This explicitly excludes `const` (ConstantReadNode/ConstantPathNode), strings, symbols, etc.
fn is_acceptable_arg(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
        || node.as_call_node().is_some()
}

/// Check if the inner call returns a new array based on RuboCop's rules.
fn inner_returns_new_array(inner: &ruby_prism::CallNode<'_>) -> bool {
    let name = inner.name().as_slice();

    // ALWAYS_RETURNS_NEW_ARRAY — qualifies, but with a caveat for real blocks.
    // In RuboCop's NodePattern, when the inner call has a block, pattern 2 requires
    // `(any_block (send _ :method) ...)` — the inner send must have NO positional args.
    // If the call has both a real block (`{ }` / `do...end`) AND positional args
    // (e.g., `Parallel.map(items) { }`), it's a custom method, not Array#map.
    // However, a block_pass (`&method(:name)`, `&:sym`) is part of the arguments in
    // Parser gem's AST — it matches pattern 3 `(send _ :method ...)`, not pattern 2.
    // In Prism, `CallNode.block()` returns both BlockNode and BlockArgumentNode, so we
    // must distinguish: only reject when the block is a real BlockNode, not a BlockArgumentNode.
    if ALWAYS_RETURNS_NEW_ARRAY.contains(&name) {
        if let Some(block) = inner.block() {
            let is_real_block = block.as_block_node().is_some();
            if is_real_block && inner.arguments().is_some() {
                return false;
            }
        }
        return true;
    }

    // RETURN_NEW_ARRAY_WHEN_ARGS — only when called with exactly one arg
    // matching RuboCop's `{int lvar ivar cvar gvar send}` NodePattern.
    if RETURN_NEW_ARRAY_WHEN_ARGS.contains(&name) {
        if let Some(args) = inner.arguments() {
            let arguments = args.arguments();
            return arguments.len() == 1
                && arguments
                    .iter()
                    .next()
                    .is_some_and(|a| is_acceptable_arg(&a));
        }
        return false;
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

    fn default_enabled(&self) -> bool {
        false
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

        // RuboCop's NodePattern only matches `send` (regular `.`), not `csend` (`&.`).
        // Skip chains where either the outer or inner call uses safe navigation.
        let outer_call = node.as_call_node().unwrap();
        if outer_call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.")
        {
            return;
        }
        if chain
            .inner_call
            .call_operator_loc()
            .is_some_and(|op| op.as_slice() == b"&.")
        {
            return;
        }

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
        // RuboCop uses `any_block_type?` which matches block/numblock but NOT block_pass.
        if chain.outer_method == b"select" {
            // The receiver must be a real block call (e.g., `model.select { ... }.select { ... }`),
            // not a block_pass like `select(&:active?)`.
            let has_block_node = chain
                .inner_call
                .block()
                .and_then(|b| b.as_block_node())
                .is_some();
            let has_args = chain.inner_call.arguments().is_some();
            if !has_block_node || has_args {
                return;
            }
        }

        let inner_name = String::from_utf8_lossy(chain.inner_method);
        let outer_name = String::from_utf8_lossy(chain.outer_method);

        // Point diagnostic at the outer method name (RuboCop uses node.loc.selector)
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
