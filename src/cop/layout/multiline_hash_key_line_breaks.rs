use crate::cop::shared::node_type::{HASH_NODE, KEYWORD_HASH_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FP investigation (2026-03-08): 10 FPs, 0 FNs.
///
/// Root cause: The cop used pairwise comparison (prev.end_line == curr.start_line)
/// to detect elements sharing a line. RuboCop uses a `last_seen_line` algorithm
/// where offending elements (those that share a starting line with a predecessor)
/// do NOT update `last_seen_line`. This matters when an offending element has a
/// multiline value: its end_line is on a later line, but since it was already
/// flagged as an offense, `last_seen_line` stays at the earlier non-offending
/// element's last_line. The next element, starting on the offending element's
/// end_line, is then compared against the earlier `last_seen_line` and found to
/// be on a new line — no offense.
///
/// Fix: Replaced pairwise prev/curr comparison with RuboCop's `last_seen_line`
/// tracking from `MultilineElementLineBreaks#check_line_breaks`.
///
/// FP investigation (2026-03-25): 6 FPs, 0 FNs.
///
/// Root cause: NOT a cop logic bug. All 6 FPs come from one repo (noosfero),
/// one file (`vendor/plugins/xss_terminate/lib/html5lib_sanitize.rb`) that uses
/// hash rocket syntax (`{:key => val}`). The corpus baseline config uses
/// `TargetRubyVersion: 4.0`, and the Parser gem's Ruby 4.0 grammar cannot parse
/// `=>` in hash literals (conflicts with pattern matching syntax). RuboCop only
/// reports `Lint/Syntax` errors for this file, while Prism parses it successfully,
/// causing nitrocop to report 6 legitimate offenses that appear as FPs. Local
/// testing confirms RuboCop DOES flag the same patterns with Ruby 3.3 parser.
/// The fix belongs at the infrastructure level (e.g., `repo_excludes.json` or
/// filtering FPs where RuboCop only has `Lint/Syntax` errors), not in the cop.
pub struct MultilineHashKeyLineBreaks;

impl Cop for MultilineHashKeyLineBreaks {
    fn name(&self) -> &'static str {
        "Layout/MultilineHashKeyLineBreaks"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[HASH_NODE, KEYWORD_HASH_NODE]
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

        // Skip keyword hashes (no braces)
        if node.as_keyword_hash_node().is_some() {
            return;
        }

        let hash = match node.as_hash_node() {
            Some(h) => h,
            None => return,
        };

        let opening = hash.opening_loc();
        let closing = hash.closing_loc();

        if opening.as_slice() != b"{" || closing.as_slice() != b"}" {
            return;
        }

        let (open_line, _) = source.offset_to_line_col(opening.start_offset());
        let (close_line, _) = source.offset_to_line_col(closing.start_offset());

        // Only check multiline hashes
        if open_line == close_line {
            return;
        }

        let elements: Vec<ruby_prism::Node<'_>> = hash.elements().iter().collect();
        if elements.len() < 2 {
            return;
        }

        // Check if all elements are on the same line (RuboCop's all_on_same_line? check)
        let first_start_line = source
            .offset_to_line_col(elements[0].location().start_offset())
            .0;
        let last = elements.last().unwrap();

        if allow_multiline_final {
            // ignore_last: true — check first.first_line == last.first_line
            // (all elements start on the same line; last element can span multiple lines)
            let last_start_line = source.offset_to_line_col(last.location().start_offset()).0;
            if first_start_line == last_start_line {
                return;
            }
        } else {
            // Default: check first.first_line == last.last_line
            // (all elements fit entirely on the same line)
            let last_end_line = source
                .offset_to_line_col(last.location().end_offset().saturating_sub(1))
                .0;
            if first_start_line == last_end_line {
                return;
            }
        }

        // Track last_line of the most recent non-offending element (matches RuboCop's
        // last_seen_line algorithm). When an element is flagged, last_seen_line is NOT
        // updated, so subsequent elements are compared against the last "good" element.
        // This avoids FPs where an element starts on the same line as a preceding
        // multiline value's closing brace but on a different line from the last
        // non-offending element.
        let mut last_seen_line: isize = -1;
        for elem in &elements {
            let (start_line, start_col) = source.offset_to_line_col(elem.location().start_offset());
            if last_seen_line >= start_line as isize {
                diagnostics.push(self.diagnostic(
                    source,
                    start_line,
                    start_col,
                    "Each item in a multi-line hash must start on a separate line.".to_string(),
                ));
            } else {
                let end_line = source
                    .offset_to_line_col(elem.location().end_offset().saturating_sub(1))
                    .0;
                last_seen_line = end_line as isize;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        MultilineHashKeyLineBreaks,
        "cops/layout/multiline_hash_key_line_breaks"
    );
}
