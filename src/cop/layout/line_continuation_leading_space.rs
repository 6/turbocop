use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=5.
///
/// Verified FN shapes:
/// - Interpolated head + one plain continued tail with a leading space on the
///   second line, e.g. the `fpm`/`elasticsearch-rails` message builders.
/// - Receiver-of-`+` continuations like `"...\n\n" \ "  " + rows.join(...)`
///   and the `rails` `" HTTP_FORWARDED=" + ...` chain.
///
/// Attempted fix 1 removed the `+`-receiver skip and the mixed-fragment
/// trailing-style skip. That satisfied the new fixture cases but regressed the
/// corpus gate: expected 1,174, actual 1,202, raw delta +33, file-drop noise
/// 21, adjusted excess 12. The new excess concentrated in long interpolated
/// warning/message chains such as `jsonapi-resources`, `chefspec`, and
/// `overcommit`, which RuboCop leaves alone.
///
/// Attempted fix 2 narrowed the skip using source-line heuristics for those
/// warning/message chains. That removed the FP regression but over-skipped badly
/// on the corpus: expected 1,174, actual 844, missing 330. The heuristic
/// suppressed many legitimate offenses beyond the targeted warning patterns.
///
/// ## Fix (2026-03-14)
///
/// Root cause: `should_skip_trailing_style` used `parts.len() >= 2` which
/// skipped ALL implicit-concat dstr nodes with an interpolated head followed by
/// plain string tails. This suppressed genuine offenses on 2-part cases (one
/// interpolated head + one plain tail), such as `fpm`'s
/// `"...dependency '#{name}'...packages " \ " don't work..."` pattern.
///
/// Fix: changed threshold from `parts.len() >= 2` to `parts.len() >= 3`.
/// Two-part cases (the FN pattern) are now checked normally. Three-or-more-part
/// chains (long interpolated message builders like `chefspec`/`overcommit`) are
/// still skipped to avoid the FP regression seen in attempted fix 1.
///
/// ## Fix (2026-03-15)
///
/// Remaining FN=2: dstr nodes that are receivers of `+` were unconditionally
/// skipped via `in_plus_receiver` flag. RuboCop's `on_dstr` has no such skip —
/// it processes all dstr nodes regardless of whether they're `+` receivers.
///
/// Examples: `%Q{...#{x}...} \ "\n\n" \ "  " + items.join(...)` (chefspec)
/// and `"..." \ "..." \ " HTTP_FORWARDED=" + req.forwarded...` (rails).
///
/// Fix: removed the `in_plus_receiver` mechanism entirely. The
/// `should_skip_trailing_style` heuristic (interpolated head + 3+ plain tails
/// with trailing whitespace) still prevents FPs on long message builder chains.
/// Moved the former no_offense `+`-receiver test case to offense since RuboCop
/// does flag leading spaces in dstr nodes even when they're `+` receivers.
///
/// ## Fix (2026-03-15, round 2) — chefspec FP=3
///
/// CI reported FP=3 in chefspec (`resource_matcher.rb:77/78/80`):
/// `%Q{expected "#{name}[#{id}]"} \ " with action :#{act}..." \ ...`
///
/// Root cause: RuboCop's `investigate_trailing_style` autocorrect block uses
/// `first_line[LINE_1_ENDING]` where `LINE_1_ENDING = /['"]\s*\\\n/`. When
/// the first line of a continuation pair ends with a non-quote character
/// (e.g., `} \` from `%Q{...}`), the regex returns nil and the autocorrect
/// block crashes (`nil.length`). RuboCop's error handler catches the crash
/// and aborts the entire `on_dstr` processing, so no offenses are recorded
/// for that dstr node.
///
/// Fix: before calling `check_trailing_style`, verify the first line ends
/// with a quote before `\` via `first_line_ends_with_quote_before_backslash`.
/// If not AND the second line would trigger an offense (leading spaces after
/// opening quote), break the loop to match RuboCop's crash-stops-processing
/// behavior. If the second line wouldn't trigger an offense, continue to the
/// next pair (matching RuboCop's early return from `investigate_trailing_style`
/// before reaching the crash point).
pub struct LineContinuationLeadingSpace;

impl Cop for LineContinuationLeadingSpace {
    fn name(&self) -> &'static str {
        "Layout/LineContinuationLeadingSpace"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = LineContinuationVisitor {
            cop: self,
            source,
            lines: source.lines().collect(),
            enforced_style: config.get_str("EnforcedStyle", "trailing"),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct LineContinuationVisitor<'a> {
    cop: &'a LineContinuationLeadingSpace,
    source: &'a SourceFile,
    lines: Vec<&'a [u8]>,
    enforced_style: &'a str,
    diagnostics: Vec<Diagnostic>,
}

impl LineContinuationVisitor<'_> {
    fn check_dstr(&mut self, node: &ruby_prism::InterpolatedStringNode<'_>) {
        if node
            .opening_loc()
            .is_some_and(|opening| opening.as_slice().starts_with(b"<<"))
        {
            return;
        }

        let loc = node.location();
        let (start_line, _) = self.source.offset_to_line_col(loc.start_offset());
        let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
        let (end_line, _) = self.source.offset_to_line_col(end_offset);
        if start_line == end_line {
            return;
        }

        if self.lines.get(start_line - 1..end_line).is_none() {
            return;
        }
        let parts: Vec<_> = node.parts().iter().collect();
        let skip_trailing_style = self.enforced_style != "leading"
            && should_skip_trailing_style(node, &parts, trim_cr(self.lines[start_line - 1]));

        for idx in 0..end_line.saturating_sub(start_line) {
            let line_num = start_line + idx;
            let first_line = trim_cr(self.lines[start_line - 1 + idx]);
            if !first_line.ends_with(b"\\") || !self.continuation(node, line_num) {
                continue;
            }

            let second_line = trim_cr(self.lines[start_line + idx]);
            match self.enforced_style {
                "leading" => self.check_leading_style(first_line, line_num),
                _ => {
                    if skip_trailing_style {
                        continue;
                    }
                    if !first_line_ends_with_quote_before_backslash(first_line) {
                        // RuboCop's autocorrect block crashes when
                        // first_line doesn't match LINE_1_ENDING (no
                        // quote before `\`), but only if second_line
                        // would trigger an offense (leading spaces after
                        // opening quote). The crash kills the entire
                        // on_dstr processing. If second_line wouldn't
                        // trigger an offense, RuboCop returns early from
                        // investigate_trailing_style without reaching the
                        // crash, and subsequent pairs are still checked.
                        if would_trigger_trailing_offense(second_line) {
                            break;
                        }
                        continue;
                    }
                    self.check_trailing_style(second_line, line_num + 1);
                }
            }
        }
    }

    fn continuation(&self, node: &ruby_prism::InterpolatedStringNode<'_>, line_num: usize) -> bool {
        node.parts().iter().all(|part| {
            let loc = part.location();
            let (start_line, _) = self.source.offset_to_line_col(loc.start_offset());
            let end_offset = loc.end_offset().saturating_sub(1).max(loc.start_offset());
            let (end_line, _) = self.source.offset_to_line_col(end_offset);
            !(start_line <= line_num && line_num < end_line)
        })
    }

    fn check_trailing_style(&mut self, line: &[u8], line_num: usize) {
        let Some(quote_idx) = line.iter().position(|b| !is_horizontal_whitespace(*b)) else {
            return;
        };
        if !matches!(line[quote_idx], b'\'' | b'"') {
            return;
        }

        let leading_len = line[quote_idx + 1..]
            .iter()
            .take_while(|b| is_horizontal_whitespace(**b))
            .count();
        if leading_len == 0 {
            return;
        }

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line_num,
            quote_idx + 1,
            "Move leading spaces to the end of the previous line.".to_string(),
        ));
    }

    fn check_leading_style(&mut self, line: &[u8], line_num: usize) {
        let Some(backslash_idx) = line.iter().rposition(|b| *b == b'\\') else {
            return;
        };

        let before_backslash = &line[..backslash_idx];
        let Some(quote_idx) = before_backslash
            .iter()
            .rposition(|b| !is_horizontal_whitespace(*b))
        else {
            return;
        };
        if !matches!(before_backslash[quote_idx], b'\'' | b'"') {
            return;
        }

        let trailing = &before_backslash[..quote_idx];
        let Some(space_start) = trailing
            .iter()
            .rposition(|b| !is_horizontal_whitespace(*b))
            .map(|idx| idx + 1)
            .or_else(|| (!trailing.is_empty()).then_some(0))
        else {
            return;
        };
        if space_start == quote_idx {
            return;
        }

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line_num,
            space_start,
            "Move trailing spaces to the start of the next line.".to_string(),
        ));
    }
}

impl<'pr> Visit<'pr> for LineContinuationVisitor<'_> {
    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        self.check_dstr(node);
        ruby_prism::visit_interpolated_string_node(self, node);
    }
}

fn trim_cr(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn is_horizontal_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t')
}

/// Returns true if the line would trigger a trailing-style offense — i.e.,
/// starts with optional whitespace, a quote, then one or more spaces.
/// Mirrors the logic in `check_trailing_style` without emitting diagnostics.
fn would_trigger_trailing_offense(line: &[u8]) -> bool {
    let Some(quote_idx) = line.iter().position(|b| !is_horizontal_whitespace(*b)) else {
        return false;
    };
    if !matches!(line[quote_idx], b'\'' | b'"') {
        return false;
    }
    line[quote_idx + 1..]
        .iter()
        .take_while(|b| is_horizontal_whitespace(**b))
        .count()
        > 0
}

/// Returns true if the line ends with `['"] \s* \\` — i.e., a standard quote
/// delimiter before the backslash continuation. Returns false for percent
/// strings like `%Q{...} \` where the line ends with `} \`.
///
/// RuboCop's `LINE_1_ENDING` regex (`/['"]\s*\\\n/`) requires a quote before
/// the backslash. When it doesn't match, RuboCop's autocorrect block crashes
/// (nil.length), killing the entire `on_dstr` processing. We replicate this by
/// breaking the loop when the first line lacks a quote ending.
fn first_line_ends_with_quote_before_backslash(line: &[u8]) -> bool {
    let Some(backslash_idx) = line.iter().rposition(|b| *b == b'\\') else {
        return false;
    };
    let before_backslash = &line[..backslash_idx];
    before_backslash
        .iter()
        .rev()
        .find(|b| !is_horizontal_whitespace(**b))
        .is_some_and(|b| matches!(b, b'\'' | b'"'))
}

fn should_skip_trailing_style(
    node: &ruby_prism::InterpolatedStringNode<'_>,
    parts: &[ruby_prism::Node<'_>],
    first_line: &[u8],
) -> bool {
    node.opening_loc().is_none()
        && parts.len() >= 3
        && parts[0].as_interpolated_string_node().is_some()
        && parts[1..]
            .iter()
            .all(|part| part.as_string_node().is_some())
        && has_trailing_whitespace_before_closing_quote(first_line)
}

fn has_trailing_whitespace_before_closing_quote(line: &[u8]) -> bool {
    let Some(backslash_idx) = line.iter().rposition(|b| *b == b'\\') else {
        return false;
    };

    let before_backslash = &line[..backslash_idx];
    let Some(quote_idx) = before_backslash
        .iter()
        .rposition(|b| !is_horizontal_whitespace(*b))
    else {
        return false;
    };
    if !matches!(before_backslash[quote_idx], b'\'' | b'"') {
        return false;
    }

    before_backslash[..quote_idx]
        .last()
        .is_some_and(|b| is_horizontal_whitespace(*b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        LineContinuationLeadingSpace,
        "cops/layout/line_continuation_leading_space"
    );

    #[test]
    fn leading_style_flags_trailing_whitespace() {
        use crate::testutil::run_cop_full_with_config;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("leading".into()),
            )]),
            ..CopConfig::default()
        };

        let diags = run_cop_full_with_config(
            &LineContinuationLeadingSpace,
            b"x = 'too ' \\\n    'long'\n",
            config,
        );

        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].location.line, 1);
        assert_eq!(diags[0].location.column, 8);
        assert_eq!(
            diags[0].message,
            "Move trailing spaces to the start of the next line."
        );
    }
}
