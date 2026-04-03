use crate::cop::shared::node_type::{CALL_NODE, LAMBDA_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for methods called on a do...end block.
///
/// ## Investigation notes
///
/// Root causes of corpus mismatches:
/// 1. **Location mismatch (FP+FN pairs):** Offense was reported at the chained
///    method name (`call.message_loc()`), but RuboCop reports at the `end` keyword
///    of the block (`receiver.loc.end.begin_pos`). When the chained method is on a
///    different line from `end` (e.g., `end\n  .join`), this produced a paired FP
///    (wrong line) and FN (missing the correct line). Fixed by reporting at the
///    block's `closing_loc` (the `end` keyword).
/// 2. **Lambda do..end blocks (FN):** `-> do ... end.call` was not detected because
///    the receiver is a `LambdaNode`, not a `CallNode`. RuboCop treats lambdas as
///    block types. Fixed by also checking for `LambdaNode` receivers.
/// 3. **Block argument false skip (FN):** `a do b end.c(&blk)` was skipped because
///    `call.block().is_some()` is true for `BlockArgumentNode`. RuboCop only ignores
///    literal blocks (where `on_block` fires). Fixed by checking that the outer
///    call's block is specifically a `BlockNode`, not a `BlockArgumentNode`.
pub struct MethodCalledOnDoEndBlock;

impl Cop for MethodCalledOnDoEndBlock {
    fn name(&self) -> &'static str {
        "Style/MethodCalledOnDoEndBlock"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, LAMBDA_NODE]
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

        // Skip if this call itself has a literal block (to avoid double-reporting
        // with MultilineBlockChain). Only skip for BlockNode, not BlockArgumentNode.
        if let Some(block) = call.block() {
            if block.as_block_node().is_some() {
                return;
            }
        }

        // Check if the receiver is a call with a do...end block, or a lambda
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Try to get a do...end block from the receiver.
        // The receiver can be:
        // 1. A CallNode with a BlockNode child (e.g., `items.each do ... end`)
        // 2. A LambdaNode (e.g., `-> do ... end`)
        let closing_loc = if let Some(recv_call) = receiver.as_call_node() {
            let block = match recv_call.block() {
                Some(b) => b,
                None => return,
            };
            let block_node = match block.as_block_node() {
                Some(b) => b,
                None => return,
            };
            // Must be a do...end block
            let opening_loc = block_node.opening_loc();
            if opening_loc.as_slice() != b"do" {
                return;
            }
            block_node.closing_loc()
        } else if let Some(lambda_node) = receiver.as_lambda_node() {
            // Lambda with do...end: -> do ... end.call
            let opening_loc = lambda_node.opening_loc();
            if opening_loc.as_slice() != b"do" {
                return;
            }
            lambda_node.closing_loc()
        } else {
            return;
        };

        // Report at the `end` keyword position (matching RuboCop's behavior)
        let (line, column) = source.offset_to_line_col(closing_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid chaining a method call on a do...end block.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MethodCalledOnDoEndBlock,
        "cops/style/method_called_on_do_end_block"
    );
}
