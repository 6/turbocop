use crate::cop::shared::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Check if a call node has a block whose body's last statement is a select/filter call.
/// Returns the select/filter CallNode and its name if found.
/// This handles the pattern: `something { ... select(&:foo) }.map(&:bar)`
fn find_select_in_block_body<'a>(
    call: &ruby_prism::CallNode<'a>,
) -> Option<(ruby_prism::CallNode<'a>, &'static str)> {
    let block = call.block()?;
    let block_node = block.as_block_node()?;
    let body = block_node.body()?;
    let stmts = body.as_statements_node()?;
    let body_stmts: Vec<_> = stmts.body().iter().collect();
    let last = body_stmts.last()?;
    let last_call = last.as_call_node()?;
    // Only match when select/filter does NOT have a real block (do/end or { }).
    // RuboCop's parent chain: select_node → parent(block) → parent.parent(call).
    // When select has a real block { }, the parent is the block node wrapping
    // select+block, so parent.parent is the outer block body — not a call node.
    // Only block_pass (select(&:foo)) works because the send node's parent is
    // the outer block directly.
    // In Prism, .block() returns Some for both BlockNode ({ }) and BlockArgumentNode (&foo),
    // so check specifically for BlockNode.
    if last_call
        .block()
        .is_some_and(|b| b.as_block_node().is_some())
    {
        return None;
    }
    let name = last_call.name().as_slice();
    if name == b"select" {
        Some((last_call, "select"))
    } else if name == b"filter" {
        Some((last_call, "filter"))
    } else {
        None
    }
}

/// Performance/SelectMap — flags `select.map` / `filter.map` chains, suggests `filter_map`.
///
/// ## Investigation (2026-03-04)
/// Single FN in corpus: `select.map { |e| ... }` where `select` is called without a block
/// (returns an Enumerator). RuboCop's guard only skips when `select` has non-block-pass
/// arguments (e.g., `select(key: value)`); bare `select` with no args passes through.
/// nitrocop was too strict — it required `inner_call.block()` to be present. Fixed to match
/// RuboCop: allow no-argument calls and block-pass args, only skip non-block-pass arguments.
///
/// ## Investigation (2026-03-22, extended corpus)
/// FN: `flat_map { |g| g.users.select(&:active?) }.map(&:name)` — select is the last
/// expression inside a block body, and .map is chained on the block result. RuboCop's
/// `map_method_candidate` handles this via `parent.block_type? && parent.parent&.call_type?`.
/// Added `find_select_in_block_body` to check if the receiver call's block body ends with
/// select/filter.
pub struct SelectMap;

impl Cop for SelectMap {
    fn name(&self) -> &'static str {
        "Performance/SelectMap"
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

        if chain.outer_method != b"map" {
            return;
        }

        // Try direct chain first: receiver.select/filter.map
        // If the inner method is not select/filter, try the block-body pattern:
        // receiver.something { ... select/filter }.map
        let inner = chain.inner_method;
        let (select_call, inner_name) = if inner == b"select" || inner == b"filter" {
            let name = if inner == b"select" {
                "select"
            } else {
                "filter"
            };
            (Some(chain.inner_call), name)
        } else {
            // Check if the inner call has a block whose last statement is select/filter.
            // This matches RuboCop's map_method_candidate:
            //   if parent.block_type? && parent.parent&.call_type? → parent.parent
            match find_select_in_block_body(&chain.inner_call) {
                Some((call, name)) => (Some(call), name),
                None => (None, ""),
            }
        };

        let select_call = match select_call {
            Some(c) => c,
            None => return,
        };

        // RuboCop's guard: `return if (first_argument = node.first_argument) && !first_argument.block_pass_type?`
        // Skip if select/filter has non-block-pass arguments (e.g., `select(key: value)`).
        // Allow: no arguments at all (bare `select.map`) or block-pass (`select(&:foo).map`).
        if let Some(args) = select_call.arguments() {
            let arg_list = args.arguments();
            if !arg_list.is_empty() {
                let first = arg_list.iter().next().unwrap();
                if first.as_block_argument_node().is_none() {
                    return;
                }
            }
        }

        // If there's a block on the select call, check for numblock/it patterns.
        // RuboCop's Parser gem has separate `block` and `numblock` node types.
        // `numblock` (used for _1/_2 numbered params and Ruby 3.4 `it`) returns
        // false for `block_type?`, causing RuboCop to skip these chains.
        if let Some(inner_block) = select_call.block() {
            if let Some(block_node) = inner_block.as_block_node() {
                if let Some(params) = block_node.parameters() {
                    if params.as_numbered_parameters_node().is_some()
                        || params.as_it_parameters_node().is_some()
                    {
                        return;
                    }
                }
            }
        }

        // Report at the select/filter method name to match RuboCop's offense_range
        let loc = select_call
            .message_loc()
            .unwrap_or_else(|| select_call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `filter_map` instead of `{inner_name}.map`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SelectMap, "cops/performance/select_map");
}
