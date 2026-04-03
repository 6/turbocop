use crate::cop::shared::node_type::{BLOCK_NODE, CALL_NODE, IF_NODE, STATEMENTS_NODE, UNLESS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/MapCompactWithConditionalBlock
///
/// Detects `map { ... }.compact` and `filter_map { ... }` patterns where the
/// block body is a conditional that returns the block parameter or nil/next,
/// suggesting replacement with `select` or `reject`.
///
/// ## Investigation findings (2026-03-23)
///
/// FN root causes:
/// - `unless` nodes were not handled (Prism uses separate UnlessNode)
/// - Guard clause patterns (`next if cond; item`) were not detected
/// - `next` in if/else branches was not recognized
/// - `filter_map` blocks were not checked
/// - Ternary with `next` was not detected (ternary is still an IfNode in Prism)
/// - `next item` (next with value) patterns were missed
/// - `next nil` explicit nil guard patterns were missed
///
/// FP root causes:
/// - elsif chains were not skipped (vendor checks `condition_node.parent.elsif?`)
/// - Non-parameter return values already handled via `truthy_branch_returns_param`
/// - Multi-parameter blocks (e.g., `|key, value|`) were incorrectly matched.
///   The vendor node pattern `(args $(arg _))` requires exactly one positional
///   argument. Returning just one of multiple params is NOT equivalent to
///   `select`/`reject` which return the full element. Fixed by checking that
///   the block has exactly one required parameter and no other param types.
/// - `filter_map` blocks used the wrong message ("Use `filter_map` instead of
///   `map { ... }.compact`") when the code already used `filter_map`. Fixed by
///   using a distinct message for the `filter_map` path.
/// - Ternary `cond ? param : nil` was incorrectly flagged. RuboCop's pattern
///   `{next nil?}` means "next node or absent child" — `nil?` tests Ruby nil
///   (absent), NOT a NilNode literal. `cond ? param : nil` has a NilNode in the
///   else branch which doesn't match `nil?`. Fixed by using `is_bare_next` only
///   for Patterns 2/3 (param+next or next+param), not `is_next_or_nil`.
/// - `filter_map(args)` calls with explicit arguments (e.g., `Parallel.filter_map(list)`)
///   were incorrectly matched. The vendor pattern `(call _ {:map :filter_map})`
///   requires no call arguments. Fixed by checking `call.arguments().is_none()`.
/// - `filter_map` in multi-line method chains (e.g., `receiver\n  .filter_map { ... }`)
///   reported the offense at the receiver's line instead of the `filter_map` selector
///   line, causing FP/FN pairs on adjacent lines. Fixed by using `message_loc()`
///   instead of `call.location()` for the filter_map path.
///
/// The vendor RuboCop NodePattern handles these block body shapes
/// (plus unless variants of all):
/// - `(if cond lvar {next|nil})` — if with param in then, next/nil in else
/// - `(if cond {next|nil} lvar)` — if with next/nil in then, param in else
/// - `(if cond (next lvar) {next|nil|nil?})` — next-with-value in then
/// - `(if cond {next|nil|nil?} (next lvar))` — next-with-value in else
/// - `(begin (if cond next nil?) lvar)` — guard clause with bare next
/// - `(begin (if cond (next lvar) nil?) (nil))` — guard with next-value + nil
pub struct MapCompactWithConditionalBlock;

impl Cop for MapCompactWithConditionalBlock {
    fn name(&self) -> &'static str {
        "Style/MapCompactWithConditionalBlock"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, IF_NODE, STATEMENTS_NODE, UNLESS_NODE]
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

        if method_name == b"compact" {
            // .compact call — check receiver is map/filter_map with conditional block
            if call.arguments().is_some() {
                return;
            }

            let receiver = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            let map_call = match receiver.as_call_node() {
                Some(c) => c,
                None => return,
            };

            let map_name = map_call.name().as_slice();
            if map_name != b"map" && map_name != b"filter_map" {
                return;
            }

            if let Some(block) = map_call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if check_block_body(source, &block_node) {
                        let loc = call.location();
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        let msg = if map_name == b"filter_map" {
                            "Use `select` or `reject` instead of `filter_map { ... }.compact`."
                        } else {
                            "Use `filter_map` instead of `map { ... }.compact`."
                        };
                        diagnostics.push(self.diagnostic(source, line, column, msg.to_string()));
                    }
                }
            }
        } else if method_name == b"filter_map" {
            // filter_map call — check if it has a conditional block that can be
            // replaced with select/reject.
            // RuboCop pattern: (call _ {:map :filter_map}) requires no arguments.
            // Skip calls like Parallel.filter_map(args) which have explicit arguments.
            if call.arguments().is_some() {
                return;
            }
            if let Some(block) = call.block() {
                if let Some(block_node) = block.as_block_node() {
                    if check_block_body(source, &block_node) {
                        // Use message_loc (method name position) for correct line in
                        // multi-line chains where the receiver is on a different line.
                        let loc = call.message_loc().unwrap_or(call.location());
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        diagnostics.push(self.diagnostic(
                            source,
                            line,
                            column,
                            "Replace `filter_map { ... }` with `select` or `reject`.".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

/// Check if a block body matches the conditional pattern.
/// Returns true if the block should be flagged.
fn check_block_body(source: &SourceFile, block_node: &ruby_prism::BlockNode<'_>) -> bool {
    let body = match block_node.body() {
        Some(b) => b,
        None => return false,
    };

    let stmts = match body.as_statements_node() {
        Some(s) => s,
        None => return false,
    };

    let body_nodes: Vec<_> = stmts.body().iter().collect();

    let param_name = match get_block_param_name(block_node) {
        Some(n) => n,
        None => return false,
    };

    match body_nodes.len() {
        1 => {
            // Single statement: if/unless conditional
            check_single_conditional(source, &body_nodes[0], &param_name)
        }
        2 => {
            // Two statements: guard clause pattern
            // Pattern: (if cond next/next-val) followed by lvar or nil
            check_guard_clause(source, &body_nodes[0], &body_nodes[1], &param_name)
        }
        _ => false,
    }
}

/// Check a single conditional statement (if/unless/ternary).
fn check_single_conditional(
    source: &SourceFile,
    expr: &ruby_prism::Node<'_>,
    param_name: &[u8],
) -> bool {
    if let Some(if_node) = expr.as_if_node() {
        return check_if_node(source, &if_node, param_name);
    }
    if let Some(unless_node) = expr.as_unless_node() {
        return check_unless_node(source, &unless_node, param_name);
    }
    false
}

/// Check an IfNode (covers regular if, ternary, modifier if).
fn check_if_node(source: &SourceFile, if_node: &ruby_prism::IfNode<'_>, param_name: &[u8]) -> bool {
    // Skip elsif chains
    if is_elsif(source, if_node) {
        return false;
    }

    let then_stmts = get_if_then_stmts(if_node);
    let else_stmts = get_if_else_stmts(if_node);

    // Pattern 1: if cond; param; end (no else — implicit nil)
    if else_stmts.is_none() {
        if let Some(ref then) = then_stmts {
            if then.len() == 1 && is_param_read(&then[0], param_name) {
                return true;
            }
            // `next param if cond` — modifier if with next-value, no else
            if then.len() == 1 && is_next_with_param(&then[0], param_name) {
                return true;
            }
        }
        return false;
    }

    let then_stmts = then_stmts.unwrap_or_default();
    let else_stmts = else_stmts.unwrap_or_default();

    // Pattern 2: if cond; param; else; next; end
    // RuboCop's pattern `{next nil?}` means "next node or absent" — NOT nil literal.
    // Nil literal in else (e.g., `cond ? param : nil`) does NOT match.
    if then_stmts.len() == 1
        && else_stmts.len() == 1
        && is_param_read(&then_stmts[0], param_name)
        && is_bare_next(&else_stmts[0])
    {
        return true;
    }

    // Pattern 3: if cond; next; else; param; end
    if then_stmts.len() == 1
        && else_stmts.len() == 1
        && is_bare_next(&then_stmts[0])
        && is_param_read(&else_stmts[0], param_name)
    {
        return true;
    }

    // Pattern 4: if cond; next param; else; next/nil/nil?; end
    if then_stmts.len() == 1
        && else_stmts.len() == 1
        && is_next_with_param(&then_stmts[0], param_name)
        && is_next_or_nil_or_nil_literal(&else_stmts[0])
    {
        return true;
    }

    // Pattern 5: if cond; next/nil/nil?; else; next param; end
    if then_stmts.len() == 1
        && else_stmts.len() == 1
        && is_next_or_nil_or_nil_literal(&then_stmts[0])
        && is_next_with_param(&else_stmts[0], param_name)
    {
        return true;
    }

    false
}

/// Check an UnlessNode.
fn check_unless_node(
    _source: &SourceFile,
    unless_node: &ruby_prism::UnlessNode<'_>,
    param_name: &[u8],
) -> bool {
    let then_stmts = get_unless_then_stmts(unless_node);
    let else_stmts = get_unless_else_stmts(unless_node);

    // Pattern: unless cond; param; end (no else — implicit nil, reject)
    if else_stmts.is_none() {
        if let Some(ref then) = then_stmts {
            if then.len() == 1 && is_param_read(&then[0], param_name) {
                return true;
            }
            // `next param unless cond` — modifier unless with next-value
            if then.len() == 1 && is_next_with_param(&then[0], param_name) {
                return true;
            }
        }
        return false;
    }

    let then_stmts = then_stmts.unwrap_or_default();
    let else_stmts = else_stmts.unwrap_or_default();

    // unless cond; param; else; next; end
    if then_stmts.len() == 1
        && else_stmts.len() == 1
        && is_param_read(&then_stmts[0], param_name)
        && is_bare_next(&else_stmts[0])
    {
        return true;
    }

    // unless cond; next; else; param; end
    if then_stmts.len() == 1
        && else_stmts.len() == 1
        && is_bare_next(&then_stmts[0])
        && is_param_read(&else_stmts[0], param_name)
    {
        return true;
    }

    false
}

/// Check guard clause pattern: two statements where first is modifier if/unless
/// with next, and second is the return value (lvar or nil).
fn check_guard_clause(
    _source: &SourceFile,
    first: &ruby_prism::Node<'_>,
    second: &ruby_prism::Node<'_>,
    param_name: &[u8],
) -> bool {
    // Shape A: (if cond next) followed by (lvar) — bare next guard
    // Shape B: (if cond (next lvar)) followed by (nil) — next-with-value guard
    // Shape C: (if cond (next nil)) followed by (lvar) — next-nil guard

    if let Some(if_node) = first.as_if_node() {
        let then_stmts = get_if_then_stmts(&if_node);
        // modifier if: no else branch
        if if_node.subsequent().is_some() {
            return false;
        }
        if let Some(ref then) = then_stmts {
            if then.len() == 1 {
                // Shape A: `next if cond` + `param`
                if is_bare_next(&then[0]) && is_param_read(second, param_name) {
                    return true;
                }
                // Shape A2: `next nil if cond` + `param`
                if is_next_nil(&then[0]) && is_param_read(second, param_name) {
                    return true;
                }
                // Shape B: `next param if cond` + `nil`
                if is_next_with_param(&then[0], param_name) && is_nil_literal(second) {
                    return true;
                }
                // Shape B2: `next param if cond` + implicit (single statement, no second)
                // This is handled by the single-statement path
            }
        }
    }

    if let Some(unless_node) = first.as_unless_node() {
        let then_stmts = get_unless_then_stmts(&unless_node);
        // modifier unless: no else branch
        if unless_node.else_clause().is_some() {
            return false;
        }
        if let Some(ref then) = then_stmts {
            if then.len() == 1 {
                // Shape A: `next unless cond` + `param`
                if is_bare_next(&then[0]) && is_param_read(second, param_name) {
                    return true;
                }
                // Shape A2: `next nil unless cond` + `param`
                if is_next_nil(&then[0]) && is_param_read(second, param_name) {
                    return true;
                }
                // Shape B: `next param unless cond` + `nil`
                if is_next_with_param(&then[0], param_name) && is_nil_literal(second) {
                    return true;
                }
            }
        }
    }

    false
}

/// Extract the single block parameter name (e.g., `|x|` -> "x").
/// Returns `None` if the block has zero or multiple parameters, matching
/// the vendor RuboCop node pattern `(args $(arg _))` which requires exactly
/// one positional argument.
fn get_block_param_name(block_node: &ruby_prism::BlockNode<'_>) -> Option<Vec<u8>> {
    let params = block_node.parameters()?;
    let block_params = params.as_block_parameters_node()?;
    let parameters = block_params.parameters()?;
    let requireds = parameters.requireds();
    let mut iter = requireds.iter();
    let first = iter.next()?;
    // Must have exactly one required parameter
    if iter.next().is_some() {
        return None;
    }
    // Also reject if there are optional, rest, keyword, or other param types
    if parameters.optionals().iter().next().is_some()
        || parameters.rest().is_some()
        || parameters.keywords().iter().next().is_some()
        || parameters.keyword_rest().is_some()
        || parameters.block().is_some()
    {
        return None;
    }
    let req_param = first.as_required_parameter_node()?;
    Some(req_param.name().as_slice().to_vec())
}

/// Check if a node is a local variable read matching the parameter name.
fn is_param_read(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    if let Some(lvar) = node.as_local_variable_read_node() {
        return lvar.name().as_slice() == param_name;
    }
    false
}

/// Check if a node is bare `next` (no arguments).
fn is_bare_next(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(next_node) = node.as_next_node() {
        // bare next has no arguments
        return next_node.arguments().is_none();
    }
    false
}

/// Check if a node is `next nil`.
fn is_next_nil(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(next_node) = node.as_next_node() {
        if let Some(args) = next_node.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() == 1 && arg_list[0].as_nil_node().is_some() {
                return true;
            }
        }
    }
    false
}

/// Check if a node is `next` (bare), `nil` literal, or `next nil`.
fn is_next_or_nil_or_nil_literal(node: &ruby_prism::Node<'_>) -> bool {
    is_bare_next(node) || is_nil_literal(node) || is_next_nil(node)
}

/// Check if a node is a `nil` literal.
fn is_nil_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_nil_node().is_some()
}

/// Check if a node is `next param` (next with the param as argument).
fn is_next_with_param(node: &ruby_prism::Node<'_>, param_name: &[u8]) -> bool {
    if let Some(next_node) = node.as_next_node() {
        if let Some(args) = next_node.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() == 1 && is_param_read(&arg_list[0], param_name) {
                return true;
            }
        }
    }
    false
}

/// Check if an IfNode is actually an `elsif` (vs a top-level `if`).
fn is_elsif(source: &SourceFile, if_node: &ruby_prism::IfNode<'_>) -> bool {
    // Check the subsequent (else/elsif) for elsif nodes
    // Actually, we need to check if THIS if_node's condition contains an elsif.
    // The vendor checks `condition_node.parent.elsif?` which means the if_node
    // itself is an elsif clause.
    // In Prism, elsif is represented as a nested IfNode in the subsequent field.
    // We need to check if the if_node has a subsequent that is an elsif (another IfNode).
    // But more importantly, we need to check if the if_node itself has an elsif child.
    if let Some(subsequent) = if_node.subsequent() {
        if subsequent.as_if_node().is_some() {
            return true;
        }
    }

    // Also check: is this if_node's keyword "elsif"?
    if let Some(kw) = if_node.if_keyword_loc() {
        let kw_bytes = &source.content[kw.start_offset()..kw.end_offset()];
        if kw_bytes == b"elsif" {
            return true;
        }
    }

    false
}

/// Get the then-branch statements of an IfNode.
fn get_if_then_stmts<'a>(if_node: &ruby_prism::IfNode<'a>) -> Option<Vec<ruby_prism::Node<'a>>> {
    let stmts = if_node.statements()?;
    Some(stmts.body().iter().collect())
}

/// Get the else-branch statements of an IfNode.
fn get_if_else_stmts<'a>(if_node: &ruby_prism::IfNode<'a>) -> Option<Vec<ruby_prism::Node<'a>>> {
    let subsequent = if_node.subsequent()?;
    if let Some(else_node) = subsequent.as_else_node() {
        if let Some(stmts) = else_node.statements() {
            return Some(stmts.body().iter().collect());
        }
        // else with no body — treat as empty
        return Some(vec![]);
    }
    None
}

/// Get the then-branch statements of an UnlessNode.
fn get_unless_then_stmts<'a>(
    unless_node: &ruby_prism::UnlessNode<'a>,
) -> Option<Vec<ruby_prism::Node<'a>>> {
    let stmts = unless_node.statements()?;
    Some(stmts.body().iter().collect())
}

/// Get the else-branch statements of an UnlessNode.
fn get_unless_else_stmts<'a>(
    unless_node: &ruby_prism::UnlessNode<'a>,
) -> Option<Vec<ruby_prism::Node<'a>>> {
    let else_clause = unless_node.else_clause()?;
    if let Some(stmts) = else_clause.statements() {
        return Some(stmts.body().iter().collect());
    }
    Some(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MapCompactWithConditionalBlock,
        "cops/style/map_compact_with_conditional_block"
    );
}
