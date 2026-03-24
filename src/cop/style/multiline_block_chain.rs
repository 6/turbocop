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
            // Walk the receiver chain looking for a call with a multiline block
            self.check_receiver_chain(node);
        }

        // Continue traversal into children
        ruby_prism::visit_call_node(self, node);
    }
}

impl BlockChainVisitor<'_> {
    /// Check if a node is a multiline block (do..end or {..}).
    fn is_multiline_block(&self, block: &ruby_prism::Node<'_>) -> bool {
        if block.as_block_node().is_none() {
            return false;
        }
        let block_loc = block.location();
        let (block_start, _) = self.source.offset_to_line_col(block_loc.start_offset());
        let (block_end, _) = self
            .source
            .offset_to_line_col(block_loc.end_offset().saturating_sub(1));
        block_start != block_end
    }

    fn check_receiver_chain(&mut self, node: &ruby_prism::CallNode<'_>) {
        // RuboCop: node.send_node.each_node(:call) walks through ALL call nodes
        // in the method chain. For each, it checks if the receiver is a multiline
        // block. We walk the receiver chain iteratively.
        //
        // For `a do..end.c1.c2 do..end`, the chain from `.c2` is:
        //   .c2 -> receiver .c1 -> receiver (a do..end)
        // We check: does .c2's receiver (.c1) have a multiline block? No.
        // Then: does .c1's receiver (a do..end) have a multiline block? Check if
        // (a do..end) is a call with a multiline block receiver... Actually no,
        // `a` is the call, and it has a block. We need to check if `.c1`'s
        // receiver is any_block_type AND multiline.
        //
        // In Prism, `a do..end` is a CallNode with a block. The receiver of `.c1`
        // is this CallNode. We need to check if that CallNode's block is multiline.

        let mut current_call = node.receiver();
        while let Some(recv_node) = current_call {
            if let Some(recv_call) = recv_node.as_call_node() {
                // Does this call have a multiline block?
                if let Some(block) = recv_call.block() {
                    if self.is_multiline_block(&block) {
                        // Found a multiline block in the chain.
                        // RuboCop reports at receiver.loc.end.begin_pos — the closing
                        // delimiter of the block (end/}). We report at the end_offset
                        // of the block minus the closing keyword length to get its start.
                        let block_loc = block.location();
                        let end_offset = block_loc.end_offset();
                        // Find the start of the closing delimiter. For `end` it's 3 bytes
                        // back, for `}` it's 1 byte back. We can check the source byte.
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
                        // Done — if there are more blocks in the chain, they will be
                        // found by subsequent on_block (visit_call_node) calls.
                        return;
                    }
                }
                // No multiline block on this call — continue walking up the chain
                current_call = recv_call.receiver();
            } else {
                // Not a call node — stop walking
                return;
            }
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
