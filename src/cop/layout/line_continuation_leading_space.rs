use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Mirrors RuboCop's `Layout/LineContinuationLeadingSpace` for continued
/// interpolated strings.
///
/// The remaining misses came from an over-broad trailing-style skip for outer
/// implicit-concat dstr nodes with an interpolated head and plain string tails.
/// That skip suppressed real offenses in chains like:
/// `"...#{x}... " \ ' tail' \ ' tail'`.
///
/// The fix is to always inspect those continuation pairs and keep only the
/// narrow `%Q{...} \` crash parity: when the first line lacks a quote before the
/// backslash, RuboCop aborts that dstr's trailing-style processing instead of
/// reporting offenses.
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

        for idx in 0..end_line.saturating_sub(start_line) {
            let line_num = start_line + idx;
            let raw_first_line = self.lines[start_line - 1 + idx];
            if !raw_first_line.ends_with(b"\\") || !self.continuation(node, line_num) {
                continue;
            }

            let first_line = trim_cr(raw_first_line);
            let second_line = trim_cr(self.lines[start_line + idx]);
            match self.enforced_style {
                "leading" => self.check_leading_style(first_line, line_num),
                _ => {
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
