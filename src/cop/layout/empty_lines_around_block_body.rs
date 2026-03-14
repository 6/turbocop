use crate::cop::node_type::{BLOCK_NODE, LAMBDA_NODE};
use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-14)
///
/// FP=1: backslash line continuation before `do` (e.g. `method(arg) \\\n  do |x|`)
/// caused the blank line after `do` to be flagged. RuboCop uses
/// `send_node.last_line` as the reference, so the `do` line itself is the
/// "first body line" and the blank line is not adjacent to the opening.
/// Fix: walk backward through `\\`-continued lines to find the effective
/// first line of the block construct.
///
/// FN=6: lambda brace/do blocks (`-> (a) {`, `-> do`) were not checked
/// because the cop only visited `BLOCK_NODE`. Added `LAMBDA_NODE`.
/// Previous attempt (2026-03-10) regressed because it did not adjust
/// `keyword_offset` for backslash continuations simultaneously; this
/// combined fix resolves both.
pub struct EmptyLinesAroundBlockBody;

/// Walk backward from `opening_line` (1-indexed) through lines ending with
/// `\` (backslash continuation) and return the byte offset of the start of
/// the earliest continuation line. Returns the original `opening_offset`
/// unchanged when there is no continuation.
fn adjusted_keyword_offset(source: &SourceFile, opening_offset: usize) -> usize {
    let (mut line, _) = source.offset_to_line_col(opening_offset);
    loop {
        if line <= 1 {
            break;
        }
        let prev_line = line - 1;
        if let Some(prev_bytes) = util::line_at(source, prev_line) {
            // Strip trailing newline/carriage-return, then check for `\`
            let mut end = prev_bytes.len();
            while end > 0 && (prev_bytes[end - 1] == b'\n' || prev_bytes[end - 1] == b'\r') {
                end -= 1;
            }
            if end > 0 && prev_bytes[end - 1] == b'\\' {
                line = prev_line;
                continue;
            }
        }
        break;
    }
    if let Some(off) = source.line_col_to_offset(line, 0) {
        off
    } else {
        opening_offset
    }
}

impl Cop for EmptyLinesAroundBlockBody {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundBlockBody"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, LAMBDA_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "no_empty_lines");
        let (opening_offset, closing_offset) = if let Some(b) = node.as_block_node() {
            (
                b.opening_loc().start_offset(),
                b.closing_loc().start_offset(),
            )
        } else if let Some(l) = node.as_lambda_node() {
            (
                l.opening_loc().start_offset(),
                l.closing_loc().start_offset(),
            )
        } else {
            return;
        };

        // For the "beginning" check, walk backward through backslash
        // continuations so that `method(arg) \\\n  do |x|` uses the
        // method-call line as the reference (matching RuboCop's
        // send_node.last_line behavior).
        let effective_opening = adjusted_keyword_offset(source, opening_offset);

        match style {
            "empty_lines" => {
                diagnostics.extend(
                    util::check_missing_empty_lines_around_body_with_corrections(
                        self.name(),
                        source,
                        effective_opening,
                        closing_offset,
                        "block",
                        corrections,
                    ),
                );
            }
            _ => {
                diagnostics.extend(util::check_empty_lines_around_body_with_corrections(
                    self.name(),
                    source,
                    effective_opening,
                    closing_offset,
                    "block",
                    corrections,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        EmptyLinesAroundBlockBody,
        "cops/layout/empty_lines_around_block_body"
    );
    crate::cop_autocorrect_fixture_tests!(
        EmptyLinesAroundBlockBody,
        "cops/layout/empty_lines_around_block_body"
    );

    #[test]
    fn single_line_block_no_offense() {
        let src = b"[1, 2, 3].each { |x| puts x }\n";
        let diags = run_cop_full(&EmptyLinesAroundBlockBody, src);
        assert!(diags.is_empty(), "Single-line block should not trigger");
    }

    #[test]
    fn do_end_block_with_blank_lines() {
        let src = b"items.each do |x|\n\n  puts x\n\nend\n";
        let diags = run_cop_full(&EmptyLinesAroundBlockBody, src);
        assert_eq!(
            diags.len(),
            2,
            "Should flag both beginning and end blank lines"
        );
    }

    #[test]
    fn empty_lines_style_requires_blank_lines() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("empty_lines".into()),
            )]),
            ..CopConfig::default()
        };
        // Block WITHOUT blank lines at beginning/end
        let src = b"items.each do |x|\n  puts x\nend\n";
        let diags = run_cop_full_with_config(&EmptyLinesAroundBlockBody, src, config);
        assert_eq!(
            diags.len(),
            2,
            "empty_lines style should require blank lines at both ends"
        );
    }

    #[test]
    fn empty_lines_style_accepts_blank_lines() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([(
                "EnforcedStyle".into(),
                serde_yml::Value::String("empty_lines".into()),
            )]),
            ..CopConfig::default()
        };
        // Block WITH blank lines at beginning/end
        let src = b"items.each do |x|\n\n  puts x\n\nend\n";
        let diags = run_cop_full_with_config(&EmptyLinesAroundBlockBody, src, config);
        assert!(
            diags.is_empty(),
            "empty_lines style should accept blank lines"
        );
    }
}
