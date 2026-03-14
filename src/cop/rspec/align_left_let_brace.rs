use ruby_prism::Visit;

use crate::cop::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_let};
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

        // Step 2: Resolve offsets to (line, column) and filter to single-line blocks
        let mut lets: Vec<(usize, usize)> = Vec::new();
        for (open_offset, close_offset) in &collector.blocks {
            let (open_line, open_col) = source.offset_to_line_col(*open_offset);
            let (close_line, _) = source.offset_to_line_col(*close_offset);
            if open_line == close_line {
                lets.push((open_line, open_col));
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
    /// Pairs of (opening_brace_offset, closing_brace_offset)
    blocks: Vec<(usize, usize)>,
}

impl<'pr> Visit<'pr> for LetCollector {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this is a let/let! call with no receiver
        if node.receiver().is_none() && is_rspec_let(node.name().as_slice()) {
            // Check if it has a block (not a block_pass like `let(:foo, &blk)`)
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    let open_offset = block_node.opening_loc().start_offset();
                    let close_offset = block_node.closing_loc().start_offset();
                    self.blocks.push((open_offset, close_offset));
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

    #[test]
    fn debug_interpolated_string_let() {
        use ruby_prism::Visit;
        // "let("foo#{1}") { 1 }" - using actual interpolation syntax
        // In source: `let("foo#{1}") { 1 }`
        // Byte layout:
        //   0: l  1: e  2: t  3: (  4: "  5: f  6: o  7: o  8: #  9: {  10: 1  11: }  12: "  13: )  14: (space)  15: {  16: (space)  17: 1  18: (space)  19: }
        let source = b"let(\"foo\x23{1}\") { 1 }\nlet!(\"foo\x23{1}\") { 1 }\n";
        let source_str = std::str::from_utf8(source).unwrap();

        let parse_result = ruby_prism::parse(source);

        struct DebugVisitor {
            source_bytes: Vec<u8>,
            calls: Vec<(String, usize, usize)>, // name, open_offset, close_offset
        }

        impl<'pr> Visit<'pr> for DebugVisitor {
            fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
                let name = String::from_utf8_lossy(node.name().as_slice()).to_string();

                if node.receiver().is_none() && (name == "let" || name == "let!") {
                    if let Some(block) = node.block() {
                        if let Some(block_node) = block.as_block_node() {
                            let open_offset = block_node.opening_loc().start_offset();
                            let close_offset = block_node.closing_loc().start_offset();
                            self.calls.push((name, open_offset, close_offset));
                        }
                    }
                }

                ruby_prism::visit_call_node(self, node);
            }
        }

        let mut visitor = DebugVisitor {
            source_bytes: source.to_vec(),
            calls: Vec::new(),
        };
        visitor.visit(&parse_result.node());

        // Verify what we collected
        // For `let("foo#{1}") { 1 }`:
        //   - The block opening `{` is at offset 15 (column 15)
        //   - The string interpolation `{` is at offset 9 (column 9)
        // For `let!("foo#{1}") { 1 }` (starts at offset 22):
        //   - The block opening `{` is at offset 37 (column 15 since line 2 starts at offset 22)
        //   - Actually: let! is 4 chars, so `let!("foo#{1}") { 1 }` is:
        //     0: l 1: e 2: t 3: ! 4: ( 5: " 6: f 7: o 8: o 9: # 10: { 11: 1 12: } 13: " 14: ) 15: (sp) 16: { ...

        for (name, open_off, close_off) in &visitor.calls {
            let before = &source[..*open_off];
            let line_start = before
                .iter()
                .rposition(|&b| b == b'\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let col = open_off - line_start;
            let open_char = source.get(*open_off).copied().unwrap_or(0) as char;
            let _ = (name, close_off, col, open_char); // Suppress unused warnings
        }

        // The key assertion: both let and let! should produce exactly 1 block each,
        // and their open offsets should be pointing to `{` (the block brace), not `#` or `{` inside interpolation
        assert_eq!(
            visitor.calls.len(),
            2,
            "Expected 2 let blocks, got: {:?}",
            visitor.calls
        );

        let (name0, open0, _close0) = &visitor.calls[0];
        let (name1, open1, _close1) = &visitor.calls[1];

        assert_eq!(name0, "let");
        assert_eq!(name1, "let!");

        // char at open0 should be `{` (the block brace)
        assert_eq!(
            source[*open0] as char, '{',
            "open0 should be block `{{`, got {:?} at offset {}",
            source[*open0] as char, open0
        );
        assert_eq!(
            source[*open1] as char, '{',
            "open1 should be block `{{`, got {:?} at offset {}",
            source[*open1] as char, open1
        );

        // Column of block `{` for `let("foo#{1}") { 1 }`:
        // let("foo#{1}") { 1 }
        // 0123456789012345678
        //               ^ column 15
        let col0 = open0 - 0; // line starts at 0
        // Line 2 starts after the \n at position 21 (source up to line 2)
        // `let("foo#{1}") { 1 }\n` is 22 bytes (indices 0..21, \n at 21)
        let line2_start = source.iter().position(|&b| b == b'\n').unwrap() + 1;
        let col1 = open1 - line2_start;

        // Both should have the same column (15 for let, 16 for let! since `let!` is one char longer)
        // Actually:
        //   let("foo#{1}") { 1 }  -> block { at col 15
        //   let!("foo#{1}") { 1 } -> block { at col 16
        // The cops SHOULD NOT flag these as a group needing alignment since the columns differ by 1
        // because of the ! character.
        // But RuboCop DOES NOT flag them either.
        // If nitrocop flags them as a group and finds col0 != col1, it would report an offense.

        // Write result to file for inspection
        let result = format!(
            "calls: {:?}\ncol0={} col1={} line2_start={}\nchar0={:?} char1={:?}\n",
            visitor.calls, col0, col1, line2_start, source[*open0] as char, source[*open1] as char
        );
        std::fs::write("/tmp/debug_interp_result.txt", &result).ok();

        // Key: col0 and col1 differ, so the cops SHOULD group them and report an offense.
        // But RuboCop does NOT. Why?
        // The answer must be in single_line? check or let? check in RuboCop.
    }
}
