use ruby_prism::Visit;

use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_let};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/AlignLeftLetBrace
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=20, FN=80.
///
/// Root cause: text-based line scanning (`check_lines`) matched `let(` patterns
/// inside heredocs, strings, and comments, causing false positives. It also
/// missed edge cases where `let` lines had unusual formatting that didn't match
/// the simple text patterns.
///
/// Fix: converted to AST-based detection via `check_source` with a Prism visitor.
/// Now walks the full AST to find `CallNode`s with name `let`/`let!`, no receiver,
/// and a single-line `BlockNode` child — exactly matching RuboCop's
/// `root.each_node(:block).select { |node| let?(node) && node.single_line? }`
/// approach from `AlignLetBrace`. Uses `opening_loc` column for the left brace
/// position, matching RuboCop's `node.loc.begin.column`.
///
/// Also fixed message to remove trailing period (RuboCop uses "Align left let brace"
/// without period).
///
/// ## Corpus investigation (2026-03-12)
///
/// FP=1 remaining. Without example locations, root cause cannot be confirmed.
/// Possible causes: (1) numblock handling — RuboCop's `each_node(:block)` skips
/// `numblock` nodes (numbered params `_1`), but Prism's `BlockNode` covers both;
/// (2) edge case in let grouping across nested scopes. No code fix applied without
/// reproduction case.
///
/// ## Corpus investigation (2026-03-14)
///
/// FP=1, FN=0.
///
/// FP=1: rubocop__rubocop-rspec repo, spec/smoke_tests/weird_rspec_spec.rb:47.
/// Previous analysis incorrectly attributed to AllCops.Exclude. The corpus oracle
/// uses baseline_rubocop.yml which does NOT inherit repo-specific excludes.
///
/// ## Corpus investigation (2026-03-21)
///
/// FP=2, FN=0. Both FPs on weird_rspec_spec.rb:47 in rubocop-rspec and rubocop-rspec_rails.
///
/// Root cause: the legacy parser merges backslash-continued lines, so
/// `let('foo' \ 'bar') { 1 }` on physical lines 40-41 reports as line 40 in
/// RuboCop. Prism preserves physical lines, so nitrocop saw the block brace on
/// line 41. This shifted adjacent_let_chunks grouping: in RuboCop, line 40 is
/// adjacent to line 39 (making [39,40] a group), then line 44 is adjacent to
/// nothing, then line 47 starts a new singleton chunk (key=T). In nitrocop,
/// lines 41 and 45 created different chunking, causing lines 47-48 to form a
/// group (key=F) with differing brace columns → false alarm.
///
/// Fix: use the CALL's start line (not the block opening line) for grouping
/// position. This matches the legacy parser's line-merging behavior for
/// backslash continuations.
pub struct AlignLeftLetBrace;

impl Cop for AlignLeftLetBrace {
    fn name(&self) -> &'static str {
        "RSpec/AlignLeftLetBrace"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        // Step 1: Walk AST to collect opening/closing brace offsets for let blocks
        let mut collector = LetCollector { blocks: Vec::new() };
        collector.visit(&parse_result.node());

        if collector.blocks.is_empty() {
            return;
        }

        // Step 2: Resolve offsets to (line, column) and filter to single-line blocks.
        // Use the CALL's start line for grouping position (not the block's opening
        // line). The legacy parser merges backslash-continued lines, so
        // `let('foo' \ 'bar') { 1 }` (lines 40-41 in Prism) reports as line 40 in
        // RuboCop. Using call_line for grouping matches this behavior.
        let mut lets: Vec<(usize, usize)> = Vec::new();
        for (call_start, open_offset, close_offset) in &collector.blocks {
            let (call_line, _) = source.offset_to_line_col(*call_start);
            let (open_line, open_col) = source.offset_to_line_col(*open_offset);
            let (close_line, _) = source.offset_to_line_col(*close_offset);
            if open_line == close_line {
                // Block braces on same line → single-line let.
                // Use call_line for grouping (matches legacy parser line merging).
                lets.push((call_line, open_col));
            }
        }

        if lets.is_empty() {
            return;
        }

        // Step 3: Group by strictly consecutive line numbers, replicating RuboCop's
        // chunking behavior where after a gap the first let is isolated.
        let groups = chunk_adjacent_lets(&lets);

        // Step 4: Check alignment within each group
        for group in &groups {
            if group.len() >= 2 {
                let max_col = group.iter().map(|(_, c)| *c).max().unwrap_or(0);
                for &(line_num, brace_col) in group {
                    if brace_col != max_col {
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            brace_col,
                            "Align left let brace".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

/// Visitor that collects byte offsets of opening/closing braces for let/let! blocks.
struct LetCollector {
    /// Triples of (call_start_offset, opening_brace_offset, closing_brace_offset)
    blocks: Vec<(usize, usize, usize)>,
}

impl<'pr> Visit<'pr> for LetCollector {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this is a let/let! call with no receiver
        if node.receiver().is_none() && is_rspec_let(node.name().as_slice()) {
            // Check if it has a block (not a block_pass like `let(:foo, &blk)`)
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    let call_start = node.location().start_offset();
                    let open_offset = block_node.opening_loc().start_offset();
                    let close_offset = block_node.closing_loc().start_offset();
                    self.blocks.push((call_start, open_offset, close_offset));
                }
            }
        }

        // Continue visiting children to find nested let calls
        ruby_prism::visit_call_node(self, node);
    }
}

/// Replicate RuboCop's `adjacent_let_chunks` grouping: walk sorted single-line
/// lets and chunk by consecutive line numbers. After a gap, the first let is
/// isolated into its own singleton group (matching the Ruby `Enumerable#chunk`
/// behavior with the nil-reset pattern used in `align_let_brace.rb`).
fn chunk_adjacent_lets(lets: &[(usize, usize)]) -> Vec<Vec<(usize, usize)>> {
    if lets.is_empty() {
        return Vec::new();
    }

    // Compute the chunk key for each let, mirroring RuboCop's logic:
    //   last_line = nil
    //   chunk { |node| line = node.line; last_line = (line if last_line.nil? || last_line+1 == line); last_line.nil? }
    let mut keys: Vec<bool> = Vec::with_capacity(lets.len());
    let mut last_line: Option<usize> = None;

    for &(line, _) in lets {
        let is_adjacent = last_line.is_none() || last_line.is_some_and(|prev| prev + 1 == line);
        if is_adjacent {
            last_line = Some(line);
        } else {
            last_line = None;
        }
        keys.push(last_line.is_none());
    }

    // Group consecutive elements with the same key (Ruby's Enumerable#chunk)
    let mut groups: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut prev_key: Option<bool> = None;

    for (i, &key) in keys.iter().enumerate() {
        if prev_key == Some(key) {
            groups.last_mut().unwrap().push(lets[i]);
        } else {
            groups.push(vec![lets[i]]);
            prev_key = Some(key);
        }
    }

    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AlignLeftLetBrace, "cops/rspec/align_left_let_brace");
}
