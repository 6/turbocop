use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, LAMBDA_NODE};

pub struct SingleLineBlockChain;

impl Cop for SingleLineBlockChain {
    fn name(&self) -> &'static str {
        "Layout/SingleLineBlockChain"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, LAMBDA_NODE]
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
        // We are looking for: receiver.method where receiver is a single-line block
        // e.g. example.select { |item| item.cond? }.join('-')
        //
        // In Prism, this is a CallNode whose receiver is a BlockNode (or LambdaNode)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must have a dot/safe-nav operator (chained call)
        let dot_loc = match call.call_operator_loc() {
            Some(loc) => loc,
            None => return,
        };

        // The receiver must be a call with a block (in Prism, blocks attach to CallNode via .block())
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let (block_open_line, block_close_line) = if let Some(recv_call) = receiver.as_call_node()
        {
            // Check if the receiver call has a block
            if let Some(block_ref) = recv_call.block() {
                if let Some(block) = block_ref.as_block_node() {
                    let open_line =
                        source.offset_to_line_col(block.opening_loc().start_offset()).0;
                    let close_line =
                        source.offset_to_line_col(block.closing_loc().start_offset()).0;
                    (open_line, close_line)
                } else {
                    return;
                }
            } else {
                return;
            }
        } else if let Some(lambda) = receiver.as_lambda_node() {
            let open_line = source.offset_to_line_col(lambda.opening_loc().start_offset()).0;
            let close_line = source.offset_to_line_col(lambda.closing_loc().start_offset()).0;
            (open_line, close_line)
        } else {
            return;
        };

        // Only flag single-line blocks
        if block_open_line != block_close_line {
            return;
        }

        // The dot must be on the same line as the block closing delimiter
        let (dot_line, dot_col) = source.offset_to_line_col(dot_loc.start_offset());
        if dot_line != block_close_line {
            return;
        }

        // If the method name is on a different line than the dot, it's already on a separate line
        if let Some(msg_loc) = call.message_loc() {
            let (msg_line, _) = source.offset_to_line_col(msg_loc.start_offset());
            if msg_line != dot_line {
                return;
            }
        }

        // Determine end of the offense range: use the method name (selector) or dot+parens
        let msg_end_col = if let Some(msg_loc) = call.message_loc() {
            let (_, end_col) = source.offset_to_line_col(msg_loc.end_offset().saturating_sub(1));
            end_col + 1
        } else if let Some(open) = call.opening_loc() {
            // No selector (e.g., `.(42)`)
            let (_, end_col) = source.offset_to_line_col(open.start_offset());
            end_col + 1
        } else {
            dot_col + dot_loc.as_slice().len()
        };

        // The offense spans from the dot to the end of the method name
        let _ = msg_end_col; // used for offset calculation in RuboCop, we just mark the dot

        diagnostics.push(self.diagnostic(
            source,
            dot_line,
            dot_col,
            "Put method call on a separate line if chained to a single line block.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        SingleLineBlockChain,
        "cops/layout/single_line_block_chain"
    );
}
