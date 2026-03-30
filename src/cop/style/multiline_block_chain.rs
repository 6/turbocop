use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation history
///
/// ### Location fix (2026-03-18)
/// Changed offense location from method name (message_loc) to the dot operator
/// (call_operator_loc) before the chained method. RuboCop reports the offense
/// at `range_between(receiver.loc.end.begin_pos, send_node.source_range.end_pos)`,
/// which starts at the block closing delimiter. The corpus comparison uses
/// line:col, and the dot is the correct location for matching.
///
/// ### Previous failed attempt (commit 38898a01, reverted f8166f95)
/// Combined TWO changes: location fix + intermediate chain walk. The chain walk
/// was too aggressive, swinging from FN=162 to FP=212. The location-only fix
/// was separated out as the safe first step.
///
/// ### Fix (2026-03-23): Location + intermediate chain walk
/// Two root causes for FP=150, FN=304:
///
/// 1. **Location mismatch (~150 FP + ~150 FN):** nitrocop reported at the dot
///    operator (`.`) which is often on the line after `end`/`}`. RuboCop reports
///    at the closing delimiter of the receiver block (`end`/`}`). When the dot
///    is on a new line after `end`, nitrocop's line was off by 1 from RuboCop,
///    creating paired FP/FN entries. Fix: report at the end offset of the
///    receiver block's closing delimiter (the `end`/`}` position).
///
/// 2. **Missing intermediate chain walk (~154 FN):** For patterns like
///    `a do..end.c1.c2 do..end`, RuboCop's `send_node.each_node(:call)` walks
///    through ALL call nodes in the send chain, finding that `.c1`'s receiver
///    is the multiline block. nitrocop only checked the immediate receiver of
///    the outer call. Fix: walk the receiver chain through non-block intermediate
///    CallNodes until we find a call whose receiver is a multiline block. We
///    stop (break) on the first match, matching RuboCop's `break` after
///    `add_offense`.
///
/// ### Fix (2026-03-30): Deep send-tree walk + LambdaNode support
/// Two root causes for FN=20:
///
/// 1. **Shallow receiver-chain walk:** The previous fix walked only the direct
///    receiver chain of CallNodes. RuboCop's `each_node(:call)` walks ALL
///    descendant call nodes in the send_node tree, including calls nested inside
///    parenthesized expressions, Hash[], if/else branches, and operator calls.
///    Fix: replaced the receiver-chain loop with a recursive `SendTreeSearcher`
///    visitor that walks receiver + arguments (but not blocks) of every CallNode
///    in the send tree.
///
/// 2. **LambdaNode not recognized as a block type:** Patterns like
///    `-> do...end.method do...end` were missed because the receiver was a
///    LambdaNode, not a CallNode with a block. RuboCop treats lambda as
///    `any_block_type?`. Fix: `SendTreeSearcher` also checks for multiline
///    LambdaNode receivers.
pub struct MultilineBlockChain;

/// Visitor that checks for multiline block chains.
/// RuboCop triggers on_block, then checks if the block's send_node
/// has a receiver that is itself a multiline block. We replicate this
/// by visiting CallNodes that have blocks and checking their receiver chain.
struct BlockChainVisitor<'a> {
    source: &'a SourceFile,
    cop_name: &'static str,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for BlockChainVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Only check calls that have a real block (do..end or {..}).
        // This matches RuboCop's on_block trigger — only block-to-block chains.
        let has_block = if let Some(block) = node.block() {
            block.as_block_node().is_some()
        } else {
            false
        };

        if has_block {
            // Deep-search the send tree (receiver + arguments) for any call
            // whose receiver is a multiline block or lambda
            self.check_send_tree(node);
        }

        // Continue traversal into children
        ruby_prism::visit_call_node(self, node);
    }
}

impl BlockChainVisitor<'_> {
    fn check_send_tree(&mut self, node: &ruby_prism::CallNode<'_>) {
        let mut searcher = SendTreeSearcher {
            source: self.source,
            found_end_offset: None,
        };
        // Start the deep search from this CallNode — checks its receiver,
        // then recursively walks receiver + arguments (not the block)
        searcher.visit_call_node(node);

        if let Some(end_offset) = searcher.found_end_offset {
            let closing_start = self.find_block_closing_start(end_offset);
            let (line, column) = self.source.offset_to_line_col(closing_start);
            self.diagnostics.push(Diagnostic {
                path: self.source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.cop_name.to_string(),
                message: "Avoid multi-line chains of blocks.".to_string(),
                corrected: false,
            });
        }
    }

    /// Find the start offset of the block's closing delimiter (`end` or `}`).
    /// The block's end_offset points just past the closing delimiter.
    fn find_block_closing_start(&self, end_offset: usize) -> usize {
        let src = self.source.as_bytes();
        if end_offset >= 3 && &src[end_offset - 3..end_offset] == b"end" {
            end_offset - 3
        } else if end_offset >= 1 && src[end_offset - 1] == b'}' {
            end_offset - 1
        } else {
            // Fallback: shouldn't normally happen
            end_offset.saturating_sub(1)
        }
    }
}

/// Sub-visitor that deeply searches a send tree for any CallNode whose
/// receiver is a multiline block (CallNode with multiline BlockNode)
/// or a multiline LambdaNode. Matches RuboCop's `each_node(:call)` walk
/// on the send_node, which traverses all descendant call nodes.
struct SendTreeSearcher<'a> {
    source: &'a SourceFile,
    found_end_offset: Option<usize>,
}

impl SendTreeSearcher<'_> {
    fn is_multiline(&self, loc: &ruby_prism::Location<'_>) -> bool {
        let (start_line, _) = self.source.offset_to_line_col(loc.start_offset());
        let (end_line, _) = self
            .source
            .offset_to_line_col(loc.end_offset().saturating_sub(1));
        start_line != end_line
    }

    /// Check if a receiver node is a multiline block (CallNode with BlockNode)
    /// or a multiline LambdaNode. Returns the end_offset of the block/lambda
    /// if matched.
    fn check_receiver(&self, receiver: &ruby_prism::Node<'_>) -> Option<usize> {
        // Check for CallNode with multiline BlockNode
        if let Some(recv_call) = receiver.as_call_node() {
            if let Some(block) = recv_call.block() {
                if block.as_block_node().is_some() && self.is_multiline(&block.location()) {
                    return Some(block.location().end_offset());
                }
            }
        }
        // Check for multiline LambdaNode
        if let Some(lambda) = receiver.as_lambda_node() {
            let loc = lambda.location();
            if self.is_multiline(&loc) {
                return Some(loc.end_offset());
            }
        }
        None
    }
}

impl<'pr> Visit<'pr> for SendTreeSearcher<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found_end_offset.is_some() {
            return;
        }
        // Check if this call's receiver is a multiline block or lambda
        if let Some(receiver) = node.receiver() {
            if let Some(offset) = self.check_receiver(&receiver) {
                self.found_end_offset = Some(offset);
                return;
            }
        }
        // Walk receiver and arguments (NOT block) to find deeper matches.
        // This matches RuboCop's each_node(:call) walking into all descendants
        // of the send_node (which excludes the block).
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
            if self.found_end_offset.is_some() {
                return;
            }
        }
        if let Some(args) = node.arguments() {
            self.visit_arguments_node(&args);
        }
    }
}

impl Cop for MultilineBlockChain {
    fn name(&self) -> &'static str {
        "Style/MultilineBlockChain"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = BlockChainVisitor {
            source,
            cop_name: self.name(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineBlockChain, "cops/style/multiline_block_chain");
}
