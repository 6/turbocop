use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-03)
///
/// Corpus oracle reported FP=3,270, FN=32,439. Four root causes identified and fixed:
/// (A) Trailing braceless KeywordHashNode not expanded into individual elements —
///     `method(key: v)` seen as 1 arg, skipped by `len < 2`. Fixed by expanding
///     last arg's `elements()` when it's a KeywordHashNode (matching RuboCop line 98).
/// (B) `AllowMultilineFinalElement` config read but stored in `_allow_multiline_final`
///     (unused). Wired into `all_on_same_line?` early return.
/// (C) Missing `all_on_same_line?` check — RuboCop returns early when all args fit on
///     one line in a multiline call. Added, matching `multiline_hash_key_line_breaks.rs`.
/// (D) Bracket assignment `[]=` not skipped (RuboCop's `return if node.method?(:[]=)`).
/// (E) Pairwise `==` replaced with `last_seen_line >= first_line` tracking.
///
/// Acceptance gate after fix: expected=60,554, actual=36,292, excess_FP=0, missing_FN=24,262.
/// +4,900 new correct detections vs CI baseline (all verified as true positives).
///
/// ## Remaining FN=24,262 (2026-03-03)
///
/// The remaining false negatives likely come from patterns not yet handled:
/// - `super` / `yield` calls (not CallNode in Prism)
/// - Complex nested call chains where the outer call lacks parens
/// - Possibly `send` vs `csend` differences in safe navigation edge cases
///
/// ## Corpus investigation (2026-03-08)
///
/// Investigated no-parens command calls and confirmed they are a major FN source.
/// A local fix that removed the `(` / `)` gate matched isolated RuboCop repros,
/// but failed corpus acceptance with hundreds of apparent FP concentrated in
/// repo-local excluded/generated files. Direct repo comparison showed RuboCop
/// suppressing those files under the corpus baseline invocation while nitrocop's
/// `--config baseline_rubocop.yml` path did not.
///
/// Result: reverted the no-parens cop-only change. A safe fix likely requires
/// config-layer parity for repo-local exclude handling under explicit `--config`
/// before re-enabling no-parens coverage here.
///
/// ## Corpus investigation (2026-03-16)
///
/// 30 FPs from block pass `&` (forwarded block) on the same line as multiline
/// keyword arguments. Root cause: In Prism, `call.block()` returns the block
/// argument separately from `call.arguments()`, but in RuboCop's Parser gem,
/// `block_pass` is included in `node.arguments`. This caused two issues:
/// (F1) The trailing KeywordHashNode was incorrectly expanded into individual
///      elements even when a block arg followed (making it not the effective
///      last argument), causing FPs on keyword args sharing a line.
/// (F2) The block argument was not included in the offsets list, so the cop
///      couldn't fire on `&` when it shared a line with the previous arg's end.
/// Fix: Check `call.block()` for `BlockArgumentNode`, skip KeywordHashNode
/// expansion when block arg is present, and append block arg to offsets.
///
/// ## Corpus investigation (2026-03-09)
///
/// Re-enabled no-parens command call support. RuboCop's `on_send` does not check
/// for parens — it handles all send/csend nodes including no-parens calls.
/// Removed the strict `(` / `)` gate and instead use argument positions to
/// determine multiline span. For `[` / `]` bracket calls, these are also handled
/// (only `[]=` is skipped, matching RuboCop). The earlier corpus FP issue from
/// the 2026-03-08 attempt was likely a config-layer problem, not a cop logic issue.
pub struct MultilineMethodArgumentLineBreaks;

impl Cop for MultilineMethodArgumentLineBreaks {
    fn name(&self) -> &'static str {
        "Layout/MultilineMethodArgumentLineBreaks"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_multiline_final = config.get_bool("AllowMultilineFinalElement", false);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Issue D: Skip bracket assignment ([]=)
        if call.name().as_slice() == b"[]=" {
            return;
        }

        // Issue F: Check for block argument (e.g., `&` or `&block`). In RuboCop's
        // Parser gem, block_pass is included in `node.arguments`, but in Prism it's
        // on `call.block()` as a separate BlockArgumentNode. We must include it in
        // our offsets to match RuboCop's behavior, and NOT expand a trailing
        // KeywordHashNode when a block arg follows (since the hash is no longer the
        // effective last argument).
        let block_arg = call
            .block()
            .and_then(|b| b.as_block_argument_node().map(|_| b));

        let args = match call.arguments() {
            Some(a) => a,
            None => {
                // No regular arguments; if there's only a block arg, nothing to check
                return;
            }
        };

        // Issue A: Expand trailing keyword hash into individual key-value pairs.
        // RuboCop treats braceless keyword hash elements as separate arguments.
        // Collect (start_offset, end_offset) pairs for each effective argument.
        let raw_args: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        let has_block_arg = block_arg.is_some();
        let mut offsets: Vec<(usize, usize)> = Vec::new();
        for (i, arg) in raw_args.iter().enumerate() {
            // Only expand trailing KeywordHashNode when it is the effective last
            // argument. When a block arg follows, the hash is NOT last in RuboCop's
            // view, so we must not expand it.
            if i == raw_args.len() - 1 && !has_block_arg {
                if let Some(kw_hash) = arg.as_keyword_hash_node() {
                    // Expand braceless keyword hash into individual elements
                    for elem in kw_hash.elements().iter() {
                        offsets
                            .push((elem.location().start_offset(), elem.location().end_offset()));
                    }
                    continue;
                }
            }
            offsets.push((arg.location().start_offset(), arg.location().end_offset()));
        }

        // Append block argument to offsets (matches RuboCop including block_pass
        // in node.arguments)
        if let Some(blk) = &block_arg {
            offsets.push((blk.location().start_offset(), blk.location().end_offset()));
        }

        if offsets.len() < 2 {
            return;
        }

        // Issue C: all_on_same_line? early return (mirrors RuboCop's MultilineElementLineBreaks mixin)
        let first_start_line = source.offset_to_line_col(offsets[0].0).0;
        let last_offsets = offsets.last().unwrap();

        if allow_multiline_final {
            // Issue B: AllowMultilineFinalElement — check first.first_line == last.first_line
            let last_start_line = source.offset_to_line_col(last_offsets.0).0;
            if first_start_line == last_start_line {
                return;
            }
        } else {
            // Default: check first.first_line == last.last_line
            let last_end_line = source
                .offset_to_line_col(last_offsets.1.saturating_sub(1))
                .0;
            if first_start_line == last_end_line {
                return;
            }
        }

        // Issue E: Replace pairwise loop with last_seen_line tracking
        // Matches RuboCop's check_line_breaks: last_seen_line >= child.first_line → offense
        let mut last_seen_line: isize = -1;
        for &(start, end) in &offsets {
            let (arg_start_line, arg_start_col) = source.offset_to_line_col(start);
            let arg_end_line = source.offset_to_line_col(end.saturating_sub(1)).0;

            if last_seen_line >= arg_start_line as isize {
                diagnostics.push(
                    self.diagnostic(
                        source,
                        arg_start_line,
                        arg_start_col,
                        "Each argument in a multi-line method call must start on a separate line."
                            .to_string(),
                    ),
                );
            } else {
                last_seen_line = arg_end_line as isize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineMethodArgumentLineBreaks,
        "cops/layout/multiline_method_argument_line_breaks"
    );
}
