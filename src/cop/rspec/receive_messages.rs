use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, STATEMENTS_NODE};
use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ReceiveMessages;

struct StubInfo {
    receiver_text: String,
    receive_msg: String,
    offset: usize,
    has_block: bool,
    has_with: bool,
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

        let mut processed = vec![false; stubs.len()];

        for i in 0..stubs.len() {
            if processed[i] || stubs[i].has_block || stubs[i].has_with {
                continue;
            }

            let mut group = vec![i];
            for j in (i + 1)..stubs.len() {
                if processed[j] || stubs[j].has_block || stubs[j].has_with {
                    continue;
                }
                if stubs[i].receiver_text == stubs[j].receiver_text {
                    group.push(j);
                }
            }

            if group.len() < 2 {
                continue;
            }

            // Check for duplicate receive messages within this group
            let mut receive_msgs: Vec<&str> = Vec::new();
            let mut has_dups = false;
            for &idx in &group {
                if receive_msgs.contains(&&*stubs[idx].receive_msg) {
                    has_dups = true;
                    break;
                }
                receive_msgs.push(&stubs[idx].receive_msg);
            }

            if has_dups {
                continue;
            }

            for &idx in &group {
                processed[idx] = true;
                let (line, column) = source.offset_to_line_col(stubs[idx].offset);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `receive_messages` instead of multiple stubs.".to_string(),
                ));
            }
        }
    }
}

fn extract_allow_receive_info(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Option<StubInfo> {
    // Pattern: allow(X).to receive(:y).and_return(z)
    // AST: CallNode(.to)
    //   receiver: CallNode(allow) with arg X
    //   arguments: [CallNode(.and_return)
    //     receiver: CallNode(receive) with arg :y
    //     arguments: [z]
    //   ]
    let to_call = node.as_call_node()?;

    if to_call.name().as_slice() != b"to" {
        return None;
    }

    // Check receiver is allow(X)
    let allow_call = to_call.receiver()?.as_call_node()?;
    if allow_call.name().as_slice() != b"allow" || allow_call.receiver().is_some() {
        return None;
    }

    // Get receiver text
    let allow_args = allow_call.arguments()?;
    let allow_arg_list: Vec<_> = allow_args.arguments().iter().collect();
    if allow_arg_list.is_empty() {
        return None;
    }
    let recv_loc = allow_arg_list[0].location();
    let receiver_text =
        std::str::from_utf8(&source.as_bytes()[recv_loc.start_offset()..recv_loc.end_offset()])
            .unwrap_or("")
            .to_string();

    // Get the argument chain: receive(:y).and_return(z)
    let to_args = to_call.arguments()?;
    let to_arg_list: Vec<_> = to_args.arguments().iter().collect();
    if to_arg_list.is_empty() {
        return None;
    }

    let arg = &to_arg_list[0];

    // Walk the argument chain to find receive() and and_return()
    let mut has_and_return = false;
    let mut has_block = false;
    let mut has_with = false;
    let mut receive_msg = String::new();

    let mut current = arg.as_call_node()?;

    loop {
        let method = current.name().as_slice();
        match method {
            b"receive" if current.receiver().is_none() => {
                // Found the receive call
                if let Some(args) = current.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if !arg_list.is_empty() {
                        let msg_loc = arg_list[0].location();
                        receive_msg = std::str::from_utf8(
                            &source.as_bytes()[msg_loc.start_offset()..msg_loc.end_offset()],
                        )
                        .unwrap_or("")
                        .to_string();
                    }
                }
                break;
            }
            b"and_return" => {
                has_and_return = true;
            }
            b"with" => {
                has_with = true;
            }
            _ => {}
        }

        if current.block().is_some() {
            has_block = true;
        }

        let recv = current.receiver()?;
        current = recv.as_call_node()?;
    }

    if !has_and_return || receive_msg.is_empty() {
        return None;
    }

    // Also check for block on the to_call itself
    if to_call.block().is_some() {
        has_block = true;
    }

    let stmt_loc = node.location();

    Some(StubInfo {
        receiver_text,
        receive_msg,
        offset: stmt_loc.start_offset(),
        has_block,
        has_with,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReceiveMessages, "cops/rspec/receive_messages");
}
