use ruby_prism::Visit;

use crate::cop::shared::util::{RSPEC_DEFAULT_INCLUDE, is_rspec_let};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// RSpec/AlignRightLetBrace
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=28, FN=148.
///
/// **Root cause:** The original text-based implementation used `check_lines` with
/// regex-like pattern matching (`trimmed.starts_with("let(")`) and `rfind('}')`
/// to locate the closing brace column. This caused:
///
/// - **FPs (28):** Text matching `let(` inside comments on consecutive lines
///   with actual let calls, causing false grouping and misalignment reports.
///   Also, `rfind('}')` could match a hash literal brace rather than the block
///   closing brace in edge cases.
///
/// - **FNs (148):** Text matching was too strict — required `let(` or `let!(`
///   immediately at start of trimmed line, missing cases where intervening
///   non-let lines (comments, blank lines, other statements) separated let
///   groups differently than the AST-based approach. The text scanner also
///   couldn't distinguish block `{}` from hash `{}` reliably, potentially
///   computing wrong columns and missing alignment violations.
///
/// **Fix:** Rewrote to use AST-based detection via `check_source` with a
/// `Visit` traversal. Finds all `CallNode`s where the method is `let`/`let!`
/// with no receiver and a single-line `BlockNode` (curly braces). Uses
/// `block.closing_loc()` for precise closing brace column. Groups by
/// consecutive source lines using the same `adjacent_let_chunks` algorithm
/// that mirrors RuboCop's `Enumerable#chunk` pattern.
///
/// ## Corpus investigation (2026-03-12)
///
/// FP=1 remaining. Fixed trailing period in message ("Align right let brace."
/// → "Align right let brace") to match RuboCop's MSG. Note: message text
/// doesn't affect corpus FP counting (count-based), so FP=1 may be from
/// a different edge case (e.g., numblock handling). Without example locations,
/// root cause cannot be confirmed.
///
/// ## Corpus investigation (2026-03-14)
///
/// FP=1, FN=0.
///
/// Previous analysis incorrectly attributed to AllCops.Exclude.
///
/// ## Corpus investigation (2026-03-21)
///
/// FP=2, FN=0. Both FPs on weird_rspec_spec.rb:47 in rubocop-rspec and rubocop-rspec_rails.
///
/// Root cause: same as AlignLeftLetBrace — the legacy parser merges backslash-
/// continued lines, so `let('foo' \ 'bar') { 1 }` reports as line 40 (not 41).
/// Using the block opening line instead of the call start line for grouping
/// shifted adjacent_let_chunks chunking, causing lines 47-48 to form a group.
///
/// Fix: use call start line for grouping position, matching legacy parser.
pub struct AlignRightLetBrace;

impl Cop for AlignRightLetBrace {
    fn name(&self) -> &'static str {
        "RSpec/AlignRightLetBrace"
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
        // Step 1: Collect all single-line let blocks with their closing brace positions
        let mut collector = LetCollector {
            source,
            lets: Vec::new(),
        };
        collector.visit(&parse_result.node());
        let lets = collector.lets;

        if lets.is_empty() {
            return;
        }

        // Step 2: Group by strictly consecutive line numbers, replicating RuboCop's
        // chunking behavior where after a gap the first let is isolated.
        let groups = chunk_adjacent_lets(&lets);

        // Step 3: Check alignment within each group
        for group in &groups {
            if group.len() >= 2 {
                let max_col = group.iter().map(|(_, c)| *c).max().unwrap_or(0);
                for &(line_num, brace_col) in group {
                    if brace_col != max_col {
                        diagnostics.push(self.diagnostic(
                            source,
                            line_num,
                            brace_col,
                            "Align right let brace".to_string(),
                        ));
                    }
                }
            }
        }
    }
}

/// Visitor that collects (line, closing_brace_column) for single-line let/let! blocks.
struct LetCollector<'a> {
    source: &'a SourceFile,
    lets: Vec<(usize, usize)>,
}

impl<'a, 'pr> Visit<'pr> for LetCollector<'a> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this is a bare let/let! call
        if node.receiver().is_none() && is_rspec_let(node.name().as_slice()) {
            // Check if it has a block (curly braces)
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    let call_loc = node.location();
                    let open_loc = block_node.opening_loc();
                    let close_loc = block_node.closing_loc();

                    let (call_line, _) = self.source.offset_to_line_col(call_loc.start_offset());
                    let (open_line, _) = self.source.offset_to_line_col(open_loc.start_offset());
                    let (close_line, close_col) =
                        self.source.offset_to_line_col(close_loc.start_offset());

                    // Single-line check: block braces on same line.
                    // Use call_line for grouping position (not block opening line).
                    // The legacy parser merges backslash-continued lines, so
                    // `let('foo' \ 'bar') { 1 }` (Prism lines 40-41) reports as
                    // line 40 in RuboCop. Using call_line matches this behavior.
                    if open_line == close_line {
                        self.lets.push((call_line, close_col));
                    }
                }
            }
        }

        // Continue traversal into child nodes
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
    crate::cop_fixture_tests!(AlignRightLetBrace, "cops/rspec/align_right_let_brace");
}
