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
/// ### Fix (2026-03-30): nested send-tree search and lambda/super receivers
/// The remaining corpus FN cluster was not a simple receiver chain. RuboCop's
/// `node.send_node.each_node(:call)` also finds descendant calls inside the
/// outer send expression, such as:
///
/// - `Hash[foo.map do ... end.compact].tap { ... }`
/// - `(items.select do ... end - ['.', '..']).map do ... end`
/// - `(traverse_files do ... end.reduce(:+) || []).group_by(...).map do ... end`
/// - `-> do ... end.should raise_error(...) do ... end`
///
/// nitrocop missed these because it only walked `CallNode` receivers and only
/// treated `CallNode + BlockNode` as a block-like receiver. The fix now:
///
/// - searches the outer send expression for descendant `CallNode`s whose
///   receiver is a multiline block-like expression;
/// - treats `LambdaNode`, `SuperNode`, and `ForwardingSuperNode` receivers with
///   attached multiline blocks the same way as multiline receiver calls; and
/// - skips nested call/super block bodies during that search so inner offenses
///   do not get re-reported as outer offenses, while still traversing lambda
///   argument bodies because RuboCop sees those calls through the outer send.
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

struct SendChainSearch<'a> {
    source: &'a SourceFile,
    found_closing_start: Option<usize>,
}

impl<'pr> Visit<'pr> for BlockChainVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Only check calls that have a real block (do..end or {..}).
        // This matches RuboCop's on_block trigger — only block-to-block chains.
        let has_block = node
            .block()
            .is_some_and(|block| block.as_block_node().is_some());

        if has_block {
            self.check_send_chain(node);
        }

        // Continue traversal into children
        ruby_prism::visit_call_node(self, node);
    }
}

impl<'pr> Visit<'pr> for SendChainSearch<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if self.found_closing_start.is_some() {
            return;
        }

        if let Some(receiver) = node.receiver() {
            if let Some(closing_start) = multiline_block_closing_start(self.source, &receiver) {
                self.found_closing_start = Some(closing_start);
                return;
            }

            self.visit(&receiver);
            if self.found_closing_start.is_some() {
                return;
            }
        }

        if let Some(arguments) = node.arguments() {
            self.visit_arguments_node(&arguments);
        }
        // Skip node.block() so nested block bodies do not create an offense for
        // the outer block call.
    }

    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {}

    fn visit_super_node(&mut self, node: &ruby_prism::SuperNode<'pr>) {
        if let Some(arguments) = node.arguments() {
            self.visit_arguments_node(&arguments);
        }
        // Skip node.block() so nested super-block bodies do not leak out.
    }

    fn visit_forwarding_super_node(&mut self, _node: &ruby_prism::ForwardingSuperNode<'pr>) {}
}

impl BlockChainVisitor<'_> {
    fn check_send_chain(&mut self, node: &ruby_prism::CallNode<'_>) {
        let mut search = SendChainSearch {
            source: self.source,
            found_closing_start: None,
        };
        search.visit_call_node(node);

        if let Some(closing_start) = search.found_closing_start {
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
}

fn multiline_block_closing_start(
    source: &SourceFile,
    receiver: &ruby_prism::Node<'_>,
) -> Option<usize> {
    if let Some(recv_call) = receiver.as_call_node() {
        let block = recv_call.block()?.as_block_node()?;
        return multiline_block_closing_loc_for_block(source, &block);
    }

    if let Some(lambda) = receiver.as_lambda_node() {
        return multiline_lambda_closing_start(source, &lambda);
    }

    if let Some(super_node) = receiver.as_super_node() {
        let block = super_node.block()?.as_block_node()?;
        return multiline_block_closing_loc_for_block(source, &block);
    }

    if let Some(forwarding_super_node) = receiver.as_forwarding_super_node() {
        let block = forwarding_super_node.block()?;
        return multiline_block_closing_loc_for_block(source, &block);
    }

    None
}

fn multiline_block_closing_loc_for_block(
    source: &SourceFile,
    block: &ruby_prism::BlockNode<'_>,
) -> Option<usize> {
    let opening_line = source
        .offset_to_line_col(block.opening_loc().start_offset())
        .0;
    let closing_line = source
        .offset_to_line_col(block.closing_loc().start_offset())
        .0;

    (opening_line != closing_line).then(|| block.closing_loc().start_offset())
}

fn multiline_lambda_closing_start(
    source: &SourceFile,
    lambda: &ruby_prism::LambdaNode<'_>,
) -> Option<usize> {
    let opening_line = source
        .offset_to_line_col(lambda.opening_loc().start_offset())
        .0;
    let closing_line = source
        .offset_to_line_col(lambda.closing_loc().start_offset())
        .0;

    (opening_line != closing_line).then(|| lambda.closing_loc().start_offset())
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
