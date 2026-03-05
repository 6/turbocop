use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/ReceiveMessages: Prefer `receive_messages` over multiple `receive` stubs
/// on the same object.
///
/// ## Investigation findings (102 FPs, 317 FNs):
///
/// ### Root causes of FPs:
/// 1. Chain-walking approach matched stubs with `.once`/`.twice`/`.ordered`/
///    `.exactly(n).times` chained after `.and_return()`. RuboCop's NodePattern
///    requires `and_return` to be the OUTERMOST call on `receive(:sym)` — any
///    chaining after `and_return` disqualifies the stub.
/// 2. Stubs with multiple `and_return` arguments (e.g., `and_return(1, 2)`) were
///    matched. RuboCop requires exactly one argument.
/// 3. Stubs with splat args (`and_return(*array)`) were matched. RuboCop excludes
///    splat arguments.
/// 4. Stubs with heredoc args were matched. RuboCop excludes heredoc returns.
///
/// ### Root causes of FNs:
/// 1. Incorrect duplicate handling: old code skipped the ENTIRE group if any
///    receive message appeared twice. RuboCop's `uniq_items` removes only the
///    items with duplicate receive args, then checks if >=2 unique items remain.
/// 2. The `.with` and `.has_block` filters were applied during grouping instead
///    of during extraction. This caused stubs with `.with` or blocks to remain
///    in the group and potentially trigger the duplicate filter, preventing valid
///    stubs from being reported.
///
/// ### Fix applied:
/// Replaced chain-walking extraction with exact structural matching:
/// `allow(X).to receive(:sym).and_return(single_non_splat_arg)`.
/// The argument to `.to` must be EXACTLY `receive(:sym).and_return(val)` — no
/// extra chaining, no `.with`, no blocks on receive, no multiple args, no splats.
/// Fixed duplicate handling to match RuboCop's `uniq_items` behavior.
pub struct ReceiveMessages;

struct StubInfo {
    receiver_text: String,
    receive_msg: String,
    offset: usize,
}

impl Cop for ReceiveMessages {
    fn name(&self) -> &'static str {
        "RSpec/ReceiveMessages"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, STATEMENTS_NODE]
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
        let block = match node.as_block_node() {
            Some(b) => b,
            None => return,
        };

        let body = match block.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let mut stubs: Vec<StubInfo> = Vec::new();

        for stmt in stmts.body().iter() {
            if let Some(info) = extract_allow_receive_info(source, &stmt) {
                stubs.push(info);
            }
        }

        // Group by receiver text
        let mut processed = vec![false; stubs.len()];

        for i in 0..stubs.len() {
            if processed[i] {
                continue;
            }

            // Collect all stubs for this receiver
            let mut group = vec![i];
            for j in (i + 1)..stubs.len() {
                if processed[j] {
                    continue;
                }
                if stubs[i].receiver_text == stubs[j].receiver_text {
                    group.push(j);
                }
            }

            if group.len() < 2 {
                continue;
            }

            // Filter out items with duplicate receive messages (RuboCop's uniq_items).
            // An item is removed if ANY other item in the group has the same receive_msg.
            let unique_group: Vec<usize> = group
                .iter()
                .copied()
                .filter(|&idx| {
                    !group.iter().any(|&other| {
                        other != idx && stubs[idx].receive_msg == stubs[other].receive_msg
                    })
                })
                .collect();

            if unique_group.len() < 2 {
                // Mark all as processed so we don't revisit
                for &idx in &group {
                    processed[idx] = true;
                }
                continue;
            }

            for &idx in &unique_group {
                processed[idx] = true;
                let (line, column) = source.offset_to_line_col(stubs[idx].offset);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `receive_messages` instead of multiple stubs.".to_string(),
                ));
            }
            // Also mark non-unique items as processed
            for &idx in &group {
                processed[idx] = true;
            }
        }
    }
}

/// Match the exact pattern: `allow(X).to receive(:sym).and_return(single_arg)`
///
/// RuboCop's NodePattern:
/// ```text
/// (send (send nil? :allow ...) :to
///   (send (send nil? :receive (sym _)) :and_return !#heredoc_or_splat?))
/// ```
///
/// This means:
/// - The statement must be a `.to` call on `allow(X)`
/// - The argument to `.to` must be EXACTLY `receive(:sym).and_return(val)`
/// - `and_return` is called directly on `receive(:sym)` — no `.with`, no other chaining
/// - `and_return` takes exactly one argument
/// - That argument is not a splat node
/// - No blocks on receive (e.g., `receive(:foo) { ... }` is excluded)
/// - No chaining after `and_return` (e.g., `.once`, `.ordered` disqualify)
fn extract_allow_receive_info(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Option<StubInfo> {
    let to_call = node.as_call_node()?;

    if to_call.name().as_slice() != b"to" {
        return None;
    }

    // Check receiver is allow(X) with no receiver (bare `allow`)
    let allow_call = to_call.receiver()?.as_call_node()?;
    if allow_call.name().as_slice() != b"allow" || allow_call.receiver().is_some() {
        return None;
    }

    // Get receiver text (the argument to allow())
    let allow_args = allow_call.arguments()?;
    let allow_arg_list: Vec<_> = allow_args.arguments().iter().collect();
    if allow_arg_list.is_empty() {
        return None;
    }
    let recv_loc = allow_arg_list[0].location();
    let receiver_text = source
        .byte_slice(recv_loc.start_offset(), recv_loc.end_offset(), "")
        .to_string();

    // Get the argument to .to — must be exactly one argument
    let to_args = to_call.arguments()?;
    let to_arg_list: Vec<_> = to_args.arguments().iter().collect();
    if to_arg_list.len() != 1 {
        return None;
    }

    let arg = &to_arg_list[0];

    // The argument must be a CallNode for `and_return`
    let and_return_call = arg.as_call_node()?;
    if and_return_call.name().as_slice() != b"and_return" {
        return None;
    }

    // `and_return` must have exactly one argument, and it must not be a splat
    let and_return_args = and_return_call.arguments()?;
    let and_return_arg_list: Vec<_> = and_return_args.arguments().iter().collect();
    if and_return_arg_list.len() != 1 {
        return None;
    }

    // Check the argument is not a splat node
    if and_return_arg_list[0].as_splat_node().is_some() {
        return None;
    }

    // Check the argument is not a heredoc (string/interpolated string with heredoc flag)
    // In Prism, heredocs are represented as StringNode or InterpolatedStringNode
    // with opening_loc containing "<<" prefix
    if is_heredoc_arg(&and_return_arg_list[0]) {
        return None;
    }

    // `and_return` must not have a block
    if and_return_call.block().is_some() {
        return None;
    }

    // The receiver of `and_return` must be `receive(:sym)` — a bare receive call
    let receive_call = and_return_call.receiver()?.as_call_node()?;
    if receive_call.name().as_slice() != b"receive" {
        return None;
    }
    // receive must have no receiver (bare function call)
    if receive_call.receiver().is_some() {
        return None;
    }
    // receive must not have a block
    if receive_call.block().is_some() {
        return None;
    }

    // receive must have exactly one argument, which must be a symbol
    let receive_args = receive_call.arguments()?;
    let receive_arg_list: Vec<_> = receive_args.arguments().iter().collect();
    if receive_arg_list.len() != 1 {
        return None;
    }
    receive_arg_list[0].as_symbol_node()?;

    let msg_loc = receive_arg_list[0].location();
    let receive_msg = source
        .byte_slice(msg_loc.start_offset(), msg_loc.end_offset(), "")
        .to_string();

    if receive_msg.is_empty() {
        return None;
    }

    let stmt_loc = node.location();

    Some(StubInfo {
        receiver_text,
        receive_msg,
        offset: stmt_loc.start_offset(),
    })
}

/// Check if a node represents a heredoc argument.
/// In Prism, heredoc strings have an opening_loc that starts with "<<".
fn is_heredoc_arg(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(str_node) = node.as_string_node() {
        if let Some(opening) = str_node.opening_loc() {
            let slice = opening.as_slice();
            return slice.starts_with(b"<<");
        }
    }
    if let Some(istr_node) = node.as_interpolated_string_node() {
        if let Some(opening) = istr_node.opening_loc() {
            let slice = opening.as_slice();
            return slice.starts_with(b"<<");
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReceiveMessages, "cops/rspec/receive_messages");
}
