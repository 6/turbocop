use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// FP/FN investigation (2026-03-10): FP=2, FN=1.
///
/// Root causes:
/// - The old implementation scanned raw source lines globally, so it treated any
///   `"...\"` + `" ..."` pair as a candidate even when the continued line was
///   outside the relevant backslash-concatenated string node. This produced FPs
///   on expressions like `"  " + values.join(...)`.
/// - Trailing style only treated literal spaces as leading whitespace, but
///   RuboCop matches generic horizontal whitespace and flags tab-prefixed
///   continued string fragments too.
///
/// Fix:
/// - Scope checks to `InterpolatedStringNode` traversal like RuboCop's `on_dstr`.
/// - Mirror RuboCop's continuation guard by skipping backslashes that are inside
///   multiline child tokens.
/// - Treat tabs the same as spaces for leading/trailing whitespace detection.
/// - Skip wrappers used as the receiver of `+` concatenation and mixed-fragment
///   cases where the previous fragment already ends with in-string whitespace.
///   Those patterns accounted for the remaining corpus-backed FPs.
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
            in_plus_receiver: false,
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
    in_plus_receiver: bool,
    diagnostics: Vec<Diagnostic>,
}

impl LineContinuationVisitor<'_> {
    fn check_dstr(&mut self, node: &ruby_prism::InterpolatedStringNode<'_>) {
        if self.in_plus_receiver {
            return;
        }
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
        let interpolated_head_plain_tail = node.opening_loc().is_none()
            && parts.len() >= 2
            && parts[0].as_interpolated_string_node().is_some()
            && parts[1..]
                .iter()
                .all(|part| part.as_string_node().is_some());
        let skip_trailing_style = self.enforced_style != "leading"
            && interpolated_head_plain_tail
            && has_trailing_whitespace_before_closing_quote(trim_cr(self.lines[start_line - 1]));

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
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let is_plus = node.name().as_slice() == b"+";

        if let Some(recv) = node.receiver() {
            let was = self.in_plus_receiver;
            self.in_plus_receiver = is_plus;
            self.visit(&recv);
            self.in_plus_receiver = was;
        }

        if let Some(args) = node.arguments() {
            let was = self.in_plus_receiver;
            self.in_plus_receiver = false;
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
            self.in_plus_receiver = was;
        }

        if let Some(block) = node.block() {
            let was = self.in_plus_receiver;
            self.in_plus_receiver = false;
            self.visit(&block);
            self.in_plus_receiver = was;
        }
    }

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
