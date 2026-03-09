use std::collections::HashSet;

use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

/// Layout/RedundantLineBreak: Checks whether certain expressions that could fit
/// on a single line are broken up into multiple lines unnecessarily.
///
/// ## Implementation approach
/// Two-phase detection:
/// - **Phase 1 (AST)**: Visits CallNode and assignment write nodes. Uses walk-down
///   with `checked_chain_ranges` to approximate RuboCop's walk-up-to-outermost behavior.
/// - **Phase 2 (text)**: Detects backslash line continuations that could be collapsed.
///
/// ## Key differences from RuboCop
/// - RuboCop walks UP from `on_send` through parent sends, convertible blocks, and
///   binary operators to find the outermost expression. Nitrocop walks DOWN and uses
///   `checked_chain_ranges` + `part_of_reported_node` to approximate this.
/// - RuboCop's `configured_to_not_be_inspected?` only skips multiline blocks
///   (`any_descendant?(node, :any_block, &:multiline?)`). Nitrocop now matches this
///   by tracking multiline vs single-line blocks separately.
/// - RuboCop's `other_cop_takes_precedence?` is conditional on
///   `Layout/SingleLineBlockChain` being enabled. Nitrocop always checks for
///   single-line blocks in chains (slightly over-conservative, causing FNs).
///
/// ## Remaining gaps (FNs)
/// - No walk-up through `AndNode`/`OrNode` (binary operators) — standalone multiline
///   `&&`/`||` expressions without assignment are not checked.
/// - Missing some write node visitors (e.g., `InstanceVariableOperatorWriteNode`).
/// - `contains_single_line_block` always fires (not conditional on SingleLineBlockChain
///   being enabled), causing FNs for single-line block patterns.
///
/// ## Fixes applied (2026-03-09)
/// - Phase 2 now checks block and unsafe ranges before reporting backslash continuations.
/// - Added `ParenthesesNode` to unsafe ranges (maps to `:begin` in Parser AST).
/// - Fixed `too_long` method chain dot check to match RuboCop's `(?=(&)?\.\w)` regex.
/// - Split block range tracking into multiline-only (`contains_multiline_block`)
///   for more accurate InspectBlocks handling.
pub struct RedundantLineBreak;

impl Cop for RedundantLineBreak {
    fn name(&self) -> &'static str {
        "Layout/RedundantLineBreak"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let inspect_blocks = config.get_bool("InspectBlocks", false);
        let max_line_length = config.get_usize("MaxLineLength", 120);

        // Collect comment line numbers (1-indexed) for the comment_within check.
        let comment_lines: HashSet<usize> = parse_result
            .comments()
            .map(|c| {
                let (line, _) = source.offset_to_line_col(c.location().start_offset());
                line
            })
            .collect();

        // Pre-collect ranges of unsafe-to-split constructs:
        // if/unless/case/begin/def nodes, heredocs, and multiline strings.
        let mut unsafe_collector = UnsafeRangeCollector { ranges: Vec::new() };
        unsafe_collector.visit(&parse_result.node());
        let unsafe_ranges = unsafe_collector.ranges;

        // Pre-collect block ranges (for InspectBlocks: false check)
        let mut block_collector = BlockRangeCollector {
            ranges: Vec::new(),
            source,
        };
        block_collector.visit(&parse_result.node());
        let block_ranges = block_collector.ranges;

        // Pre-collect single-line block ranges (for Layout/SingleLineBlockChain precedence)
        let mut sl_block_collector = SingleLineBlockCollector {
            ranges: Vec::new(),
            source,
        };
        sl_block_collector.visit(&parse_result.node());
        let single_line_block_ranges = sl_block_collector.ranges;

        // Phase 1: AST-based detection (method calls and assignments)
        let mut visitor = RedundantLineBreakVisitor {
            source,
            cop_name: self.name(),
            max_line_length,
            inspect_blocks,
            comment_lines: &comment_lines,
            unsafe_ranges: &unsafe_ranges,
            block_ranges: &block_ranges,
            single_line_block_ranges: &single_line_block_ranges,
            ast_diagnostics: Vec::new(),
            reported_starts: HashSet::new(),
            reported_ranges: Vec::new(),
            checked_chain_ranges: Vec::new(),
        };
        visitor.visit(&parse_result.node());

        let reported_starts = visitor.reported_starts;
        diagnostics.extend(visitor.ast_diagnostics);

        // Phase 2: Backslash continuation detection (existing text-based approach)
        check_backslash_continuations(
            self,
            source,
            code_map,
            max_line_length,
            inspect_blocks,
            diagnostics,
            &reported_starts,
            &unsafe_ranges,
            &block_ranges,
        );
    }
}

/// Collects byte ranges of unsafe-to-split constructs.
struct UnsafeRangeCollector {
    /// (start_offset, end_offset) of nodes that make their parent unsafe to merge.
    ranges: Vec<(usize, usize)>,
}

impl<'pr> Visit<'pr> for UnsafeRangeCollector {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        let loc = node.location();
        self.ranges.push((loc.start_offset(), loc.end_offset()));
        // Don't recurse — the whole if node is unsafe
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        let loc = node.location();
        self.ranges.push((loc.start_offset(), loc.end_offset()));
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        let loc = node.location();
        self.ranges.push((loc.start_offset(), loc.end_offset()));
    }

    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        let loc = node.location();
        self.ranges.push((loc.start_offset(), loc.end_offset()));
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let loc = node.location();
        self.ranges.push((loc.start_offset(), loc.end_offset()));
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        let loc = node.location();
        self.ranges.push((loc.start_offset(), loc.end_offset()));
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        if let Some(open) = node.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                let loc = node.location();
                self.ranges.push((loc.start_offset(), loc.end_offset()));
                return;
            }
        }
        let content = node.location().as_slice();
        if content.contains(&b'\n') {
            let loc = node.location();
            self.ranges.push((loc.start_offset(), loc.end_offset()));
        }
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        if let Some(open) = node.opening_loc() {
            if open.as_slice().starts_with(b"<<") {
                let loc = node.location();
                self.ranges.push((loc.start_offset(), loc.end_offset()));
                return;
            }
        }
        let content = node.location().as_slice();
        if content.contains(&b'\n') {
            let loc = node.location();
            self.ranges.push((loc.start_offset(), loc.end_offset()));
        }
    }

    fn visit_symbol_node(&mut self, node: &ruby_prism::SymbolNode<'pr>) {
        let content = node.location().as_slice();
        if content.contains(&b'\n') {
            let loc = node.location();
            self.ranges.push((loc.start_offset(), loc.end_offset()));
        }
    }

    fn visit_interpolated_symbol_node(&mut self, node: &ruby_prism::InterpolatedSymbolNode<'pr>) {
        let content = node.location().as_slice();
        if content.contains(&b'\n') {
            let loc = node.location();
            self.ranges.push((loc.start_offset(), loc.end_offset()));
        }
    }

    /// Multiline parenthesized groups `(...)` — maps to `:begin` in Parser AST.
    /// RuboCop's `safe_to_split?` checks
    /// `node.each_descendant(:begin, :sym).none? { |b| !b.single_line? }`.
    fn visit_parentheses_node(&mut self, node: &ruby_prism::ParenthesesNode<'pr>) {
        let content = node.location().as_slice();
        if content.contains(&b'\n') {
            let loc = node.location();
            self.ranges.push((loc.start_offset(), loc.end_offset()));
        }
        // Still recurse into children to find nested unsafe constructs
        ruby_prism::visit_parentheses_node(self, node);
    }
}

/// Collects byte ranges of block/lambda nodes, tracking whether each is multiline.
struct BlockRangeCollector<'a> {
    /// (start_offset, end_offset, is_multiline)
    ranges: Vec<(usize, usize, bool)>,
    source: &'a SourceFile,
}

impl<'pr> Visit<'pr> for BlockRangeCollector<'_> {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let loc = node.location();
        let (start_line, _) = self.source.offset_to_line_col(loc.start_offset());
        let (end_line, _) = self
            .source
            .offset_to_line_col(loc.end_offset().saturating_sub(1));
        let is_multiline = start_line != end_line;
        self.ranges
            .push((loc.start_offset(), loc.end_offset(), is_multiline));
        ruby_prism::visit_block_node(self, node);
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let loc = node.location();
        let (start_line, _) = self.source.offset_to_line_col(loc.start_offset());
        let (end_line, _) = self
            .source
            .offset_to_line_col(loc.end_offset().saturating_sub(1));
        let is_multiline = start_line != end_line;
        self.ranges
            .push((loc.start_offset(), loc.end_offset(), is_multiline));
        ruby_prism::visit_lambda_node(self, node);
    }
}

/// Collects byte ranges of single-line block nodes.
struct SingleLineBlockCollector<'a> {
    ranges: Vec<(usize, usize)>,
    source: &'a SourceFile,
}

impl<'pr> Visit<'pr> for SingleLineBlockCollector<'_> {
    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let loc = node.location();
        let (start_line, _) = self.source.offset_to_line_col(loc.start_offset());
        let (end_line, _) = self
            .source
            .offset_to_line_col(loc.end_offset().saturating_sub(1));
        if start_line == end_line {
            self.ranges.push((loc.start_offset(), loc.end_offset()));
        }
        ruby_prism::visit_block_node(self, node);
    }
}

/// AST visitor that finds multiline expressions that could fit on a single line.
struct RedundantLineBreakVisitor<'a> {
    source: &'a SourceFile,
    cop_name: &'static str,
    max_line_length: usize,
    inspect_blocks: bool,
    comment_lines: &'a HashSet<usize>,
    unsafe_ranges: &'a [(usize, usize)],
    block_ranges: &'a [(usize, usize, bool)],
    single_line_block_ranges: &'a [(usize, usize)],
    ast_diagnostics: Vec<Diagnostic>,
    reported_starts: HashSet<usize>,
    /// Byte ranges of nodes already reported, to skip descendant checks.
    reported_ranges: Vec<(usize, usize)>,
    /// Byte ranges of outermost call chain nodes that were checked (whether reported or not).
    /// Inner CallNodes within these ranges are skipped to match RuboCop's walk-up behavior.
    checked_chain_ranges: Vec<(usize, usize)>,
}

impl RedundantLineBreakVisitor<'_> {
    fn is_multiline(&self, start_offset: usize, end_offset: usize) -> bool {
        let (start_line, _) = self.source.offset_to_line_col(start_offset);
        let (end_line, _) = self
            .source
            .offset_to_line_col(end_offset.saturating_sub(1).max(start_offset));
        start_line != end_line
    }

    /// Check if combining lines of this span would exceed max_line_length.
    fn too_long(&self, start_offset: usize, end_offset: usize) -> bool {
        let (start_line, _) = self.source.offset_to_line_col(start_offset);
        let (end_line, _) = self
            .source
            .offset_to_line_col(end_offset.saturating_sub(1).max(start_offset));

        let lines: Vec<&[u8]> = self.source.lines().collect();
        let mut combined = Vec::new();
        for line_num in start_line..=end_line {
            if line_num > lines.len() {
                break;
            }
            let line = lines[line_num - 1];
            if combined.is_empty() {
                combined.extend_from_slice(line);
            } else {
                let trimmed = trim_leading_whitespace(line);
                if starts_with_method_chain_dot(trimmed) {
                    combined.extend_from_slice(trimmed);
                } else {
                    combined.push(b' ');
                    combined.extend_from_slice(trimmed);
                }
            }
        }
        // Remove backslash continuations
        combined.retain(|&b| b != b'\\');

        combined.len() > self.max_line_length
    }

    fn comment_within(&self, start_offset: usize, end_offset: usize) -> bool {
        let (start_line, _) = self.source.offset_to_line_col(start_offset);
        let (end_line, _) = self
            .source
            .offset_to_line_col(end_offset.saturating_sub(1).max(start_offset));
        self.comment_lines
            .iter()
            .any(|&line| line >= start_line && line <= end_line)
    }

    /// Check if any unsafe range is contained within (or overlaps) the given span.
    fn contains_unsafe(&self, start_offset: usize, end_offset: usize) -> bool {
        self.unsafe_ranges
            .iter()
            .any(|&(us, ue)| us >= start_offset && ue <= end_offset)
    }

    /// Check if any multiline block range is contained within the given span.
    /// This matches RuboCop's `any_descendant?(node, :any_block, &:multiline?)`.
    fn contains_multiline_block(&self, start_offset: usize, end_offset: usize) -> bool {
        self.block_ranges
            .iter()
            .any(|&(bs, be, multiline)| multiline && bs >= start_offset && be <= end_offset)
    }

    /// Check if any single-line block is contained within the given span.
    fn contains_single_line_block(&self, start_offset: usize, end_offset: usize) -> bool {
        self.single_line_block_ranges
            .iter()
            .any(|&(bs, be)| bs >= start_offset && be <= end_offset)
    }

    fn suitable_as_single_line(&self, start_offset: usize, end_offset: usize) -> bool {
        !self.too_long(start_offset, end_offset)
            && !self.comment_within(start_offset, end_offset)
            && !self.contains_unsafe(start_offset, end_offset)
    }

    fn configured_to_not_be_inspected(&self, start_offset: usize, end_offset: usize) -> bool {
        // Layout/SingleLineBlockChain takes precedence for single-line blocks in chains
        // TODO: This should be conditional on Layout/SingleLineBlockChain being enabled,
        // matching RuboCop's single_line_block_chain_enabled? check.
        if self.contains_single_line_block(start_offset, end_offset) {
            return true;
        }
        // When InspectBlocks is false (default), skip expressions containing
        // multiline blocks. This matches RuboCop's:
        //   node.any_block_type? || any_descendant?(node, :any_block, &:multiline?)
        if !self.inspect_blocks && self.contains_multiline_block(start_offset, end_offset) {
            return true;
        }
        false
    }

    /// Check if a byte offset falls within any already-reported node's range.
    fn part_of_reported_node(&self, start_offset: usize, end_offset: usize) -> bool {
        self.reported_ranges
            .iter()
            .any(|&(rs, re)| start_offset >= rs && end_offset <= re)
    }

    /// Check if a node is an inner part of a call chain that was already checked.
    /// This prevents inner CallNodes from being individually checked when the
    /// outermost CallNode in the chain was already visited (and either reported or rejected).
    /// Inner nodes may share the same start offset as the outermost (since CallNode
    /// locations include the receiver), so we check for strictly smaller end offset
    /// to identify inner nodes.
    fn part_of_checked_chain(&self, start_offset: usize, end_offset: usize) -> bool {
        self.checked_chain_ranges.iter().any(|&(cs, ce)| {
            start_offset >= cs && end_offset <= ce && (start_offset > cs || end_offset < ce)
        })
    }

    fn register_offense(&mut self, start_offset: usize, end_offset: usize) {
        let (line, col) = self.source.offset_to_line_col(start_offset);

        if self.reported_starts.contains(&line) {
            return;
        }
        self.reported_starts.insert(line);
        self.reported_ranges.push((start_offset, end_offset));

        self.ast_diagnostics.push(Diagnostic {
            path: self.source.path_str().to_string(),
            location: crate::diagnostic::Location { line, column: col },
            severity: crate::diagnostic::Severity::Convention,
            cop_name: self.cop_name.to_string(),
            message: "Redundant line break detected.".to_string(),
            corrected: false,
        });
    }
}

impl<'pr> Visit<'pr> for RedundantLineBreakVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let loc = node.location();
        let start_offset = loc.start_offset();
        let end_offset = loc.end_offset();

        if self.is_multiline(start_offset, end_offset)
            && !self.part_of_reported_node(start_offset, end_offset)
            && !self.part_of_checked_chain(start_offset, end_offset)
        {
            // This is the outermost multiline CallNode in its chain (since we
            // visit top-down and inner calls would be caught by part_of_checked_chain).
            // Record it so inner CallNodes in the chain are skipped, matching
            // RuboCop's walk-up-to-outermost behavior.
            let has_call_receiver = node.receiver().and_then(|r| r.as_call_node()).is_some();
            if has_call_receiver {
                // This node has a call chain underneath. Mark the entire range
                // so inner calls are not individually checked.
                self.checked_chain_ranges.push((start_offset, end_offset));
            }

            // Skip index access chains: hash[:foo][:bar]
            let is_index_chain = if node.name().as_slice() == b"[]" {
                node.receiver()
                    .and_then(|r| r.as_call_node())
                    .is_some_and(|r| r.name().as_slice() == b"[]")
            } else {
                false
            };

            if !is_index_chain
                && self.suitable_as_single_line(start_offset, end_offset)
                && !self.configured_to_not_be_inspected(start_offset, end_offset)
            {
                self.register_offense(start_offset, end_offset);
            }
        }

        ruby_prism::visit_call_node(self, node);
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let loc = node.location();
        self.check_assignment(loc.start_offset(), loc.end_offset());
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        let loc = node.location();
        self.check_assignment(loc.start_offset(), loc.end_offset());
        ruby_prism::visit_instance_variable_write_node(self, node);
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        let loc = node.location();
        self.check_assignment(loc.start_offset(), loc.end_offset());
        ruby_prism::visit_class_variable_write_node(self, node);
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        let loc = node.location();
        self.check_assignment(loc.start_offset(), loc.end_offset());
        ruby_prism::visit_global_variable_write_node(self, node);
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        let loc = node.location();
        self.check_assignment(loc.start_offset(), loc.end_offset());
        ruby_prism::visit_constant_write_node(self, node);
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        let loc = node.location();
        self.check_assignment(loc.start_offset(), loc.end_offset());
        ruby_prism::visit_constant_path_write_node(self, node);
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        let loc = node.location();
        self.check_assignment(loc.start_offset(), loc.end_offset());
        ruby_prism::visit_multi_write_node(self, node);
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let loc = node.location();
        self.check_assignment(loc.start_offset(), loc.end_offset());
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let loc = node.location();
        self.check_assignment(loc.start_offset(), loc.end_offset());
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let loc = node.location();
        self.check_assignment(loc.start_offset(), loc.end_offset());
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }
}

impl RedundantLineBreakVisitor<'_> {
    fn check_assignment(&mut self, start_offset: usize, end_offset: usize) {
        if !self.is_multiline(start_offset, end_offset) {
            return;
        }
        if self.part_of_reported_node(start_offset, end_offset) {
            return;
        }
        if !self.suitable_as_single_line(start_offset, end_offset) {
            return;
        }
        if self.configured_to_not_be_inspected(start_offset, end_offset) {
            return;
        }
        self.register_offense(start_offset, end_offset);
    }
}

/// Phase 2: backslash continuation detection (text-based).
#[allow(clippy::too_many_arguments)]
fn check_backslash_continuations(
    cop: &RedundantLineBreak,
    source: &SourceFile,
    code_map: &CodeMap,
    max_line_length: usize,
    inspect_blocks: bool,
    diagnostics: &mut Vec<Diagnostic>,
    already_reported: &HashSet<usize>,
    unsafe_ranges: &[(usize, usize)],
    block_ranges: &[(usize, usize, bool)],
) {
    let content = source.as_bytes();
    let lines: Vec<&[u8]> = source.lines().collect();

    let mut line_starts: Vec<usize> = Vec::with_capacity(lines.len());
    let mut offset = 0usize;
    for (i, line) in lines.iter().enumerate() {
        line_starts.push(offset);
        offset += line.len();
        if i < lines.len() - 1 || (offset < content.len() && content[offset] == b'\n') {
            offset += 1;
        }
    }

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = trim_trailing_whitespace(line);

        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        if !trimmed.ends_with(b"\\") || i + 1 >= lines.len() {
            i += 1;
            continue;
        }

        let trimmed_content = trim_leading_whitespace(trimmed);
        if trimmed_content.starts_with(b"#") {
            i += 1;
            continue;
        }

        let backslash_offset = line_starts[i] + trimmed.len() - 1;
        if !code_map.is_code(backslash_offset) {
            i += 1;
            continue;
        }

        let group_start = i;
        let mut group_end = i;
        while group_end + 1 < lines.len() {
            let t = trim_trailing_whitespace(lines[group_end]);
            if !t.ends_with(b"\\") {
                break;
            }
            let next_trimmed_content =
                trim_leading_whitespace(trim_trailing_whitespace(lines[group_end + 1]));
            if next_trimmed_content.starts_with(b"#") {
                break;
            }
            group_end += 1;
        }
        let final_line_idx = group_end + 1;
        if final_line_idx >= lines.len() {
            i = final_line_idx;
            continue;
        }

        let report_line = group_start + 1; // 1-indexed
        if already_reported.contains(&report_line) {
            i = final_line_idx + 1;
            continue;
        }

        // Check if the backslash group's byte range overlaps with any unsafe
        // construct (if/case/begin/def/heredoc/multiline-string) or block.
        // This matches RuboCop's AST-level checks that prevent collapsing
        // expressions containing these constructs.
        let group_byte_start = line_starts[group_start];
        let group_byte_end = if final_line_idx < line_starts.len() {
            line_starts[final_line_idx] + lines[final_line_idx].len()
        } else {
            content.len()
        };

        let has_unsafe = unsafe_ranges
            .iter()
            .any(|&(us, ue)| us >= group_byte_start && ue <= group_byte_end);
        if has_unsafe {
            i = final_line_idx + 1;
            continue;
        }

        // When InspectBlocks is false (default), skip backslash groups that
        // overlap with any block (single-line or multiline). This is slightly
        // more conservative than RuboCop's AST-level check, but prevents Phase 2
        // from flagging expressions that the AST phase would handle differently.
        if !inspect_blocks {
            let has_block = block_ranges
                .iter()
                .any(|&(bs, be, _)| bs < group_byte_end && be > group_byte_start);
            if has_block {
                i = final_line_idx + 1;
                continue;
            }
        }

        // Build the combined single-line version.
        let indent = leading_whitespace_len(lines[group_start]);
        let mut combined = Vec::new();
        combined.extend_from_slice(&lines[group_start][..indent]);

        for (j, bline) in lines[group_start..=group_end].iter().enumerate() {
            let t = trim_trailing_whitespace(bline);
            if t.is_empty() {
                continue;
            }
            let before_bs = trim_trailing_whitespace(&t[..t.len() - 1]);
            let content_part = trim_leading_whitespace(before_bs);

            if j == 0 {
                combined.extend_from_slice(content_part);
            } else {
                combined.push(b' ');
                combined.extend_from_slice(content_part);
            }
        }

        let final_content = trim_leading_whitespace(lines[final_line_idx]);
        if !final_content.is_empty() {
            combined.push(b' ');
            combined.extend_from_slice(trim_trailing_whitespace(final_content));
        }

        if combined.len() > max_line_length {
            i = final_line_idx + 1;
            continue;
        }

        let next_content = trim_leading_whitespace(lines[group_start + 1]);
        if next_content.starts_with(b"&&") || next_content.starts_with(b"||") {
            i = final_line_idx + 1;
            continue;
        }

        if is_string_concat_continuation(&lines, group_start, group_end) {
            i = final_line_idx + 1;
            continue;
        }

        diagnostics.push(cop.diagnostic(
            source,
            report_line,
            0,
            "Redundant line break detected.".to_string(),
        ));

        i = final_line_idx + 1;
    }
}

fn trim_trailing_whitespace(line: &[u8]) -> &[u8] {
    let mut end = line.len();
    while end > 0 && (line[end - 1] == b' ' || line[end - 1] == b'\t') {
        end -= 1;
    }
    &line[..end]
}

fn trim_leading_whitespace(line: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < line.len() && (line[start] == b' ' || line[start] == b'\t') {
        start += 1;
    }
    &line[start..]
}

fn is_string_concat_continuation(lines: &[&[u8]], group_start: usize, group_end: usize) -> bool {
    for j in group_start..group_end {
        let t = trim_trailing_whitespace(lines[j]);
        if t.is_empty() || t[t.len() - 1] != b'\\' {
            return false;
        }
        let before_bs = trim_trailing_whitespace(&t[..t.len() - 1]);
        if before_bs.is_empty() {
            return false;
        }
        let last_char = before_bs[before_bs.len() - 1];
        if last_char != b'\'' && last_char != b'"' {
            return false;
        }

        if j + 1 < lines.len() {
            let next_content = trim_leading_whitespace(lines[j + 1]);
            if next_content.is_empty() {
                return false;
            }
            let first_char = next_content[0];
            if first_char != b'\'' && first_char != b'"' {
                return false;
            }
        }
    }
    if group_end < lines.len() {
        let tail_content = trim_leading_whitespace(lines[group_end]);
        if tail_content.is_empty() {
            return false;
        }
        let first_char = tail_content[0];
        if first_char != b'\'' && first_char != b'"' {
            return false;
        }
    }
    true
}

/// Check if a trimmed line starts with a method chain dot followed by a word
/// character, matching RuboCop's `/\n\s*(?=(&)?\.\w)/` pattern.
/// Lines starting with `.operator` (like `.[]`, `.==`, `.+`) get a space
/// when joining, while `.method_name` chains get no space.
fn starts_with_method_chain_dot(trimmed: &[u8]) -> bool {
    if trimmed.starts_with(b"&.") {
        trimmed.len() > 2 && is_word_char(trimmed[2])
    } else if trimmed.starts_with(b".") {
        trimmed.len() > 1 && is_word_char(trimmed[1])
    } else {
        false
    }
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn leading_whitespace_len(line: &[u8]) -> usize {
    let mut count = 0;
    for &b in line {
        if b == b' ' || b == b'\t' {
            count += 1;
        } else {
            break;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(RedundantLineBreak, "cops/layout/redundant_line_break");
}
