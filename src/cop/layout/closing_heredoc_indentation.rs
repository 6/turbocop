use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct ClosingHeredocIndentation;

impl Cop for ClosingHeredocIndentation {
    fn name(&self) -> &'static str {
        "Layout/ClosingHeredocIndentation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = HeredocVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            call_indent_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct HeredocVisitor<'a> {
    cop: &'a ClosingHeredocIndentation,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Stack of indentation levels for enclosing CallNode lines.
    call_indent_stack: Vec<usize>,
}

impl HeredocVisitor<'_> {
    fn check_heredoc(
        &mut self,
        opening_loc: ruby_prism::Location<'_>,
        closing_loc: ruby_prism::Location<'_>,
    ) {
        let bytes = self.source.as_bytes();
        let opening = &bytes[opening_loc.start_offset()..opening_loc.end_offset()];

        // Must be a heredoc
        if !opening.starts_with(b"<<") {
            return;
        }

        // Skip simple heredocs (<<FOO without - or ~) since they have no indentation control
        let after_arrows = &opening[2..];
        if !after_arrows.starts_with(b"~") && !after_arrows.starts_with(b"-") {
            return;
        }

        // Get indentation of the opening line
        let opening_line_indent = line_indent(self.source, opening_loc.start_offset());

        // Get indentation of the closing line
        let closing_line_indent = line_indent(self.source, closing_loc.start_offset());

        // If opening and closing indentation match, no offense
        if opening_line_indent == closing_line_indent {
            return;
        }

        // If the heredoc is an argument or part of a chained call, check whether
        // the closing indentation matches the indentation of any ancestor call
        // in the parent chain (RuboCop argument_indentation_correct? logic).
        for &parent_indent in &self.call_indent_stack {
            if closing_line_indent == parent_indent {
                return;
            }
        }

        // Build the diagnostic message
        let (opening_line_num, _) = self.source.offset_to_line_col(opening_loc.start_offset());
        let lines: Vec<&[u8]> = self.source.lines().collect();
        let empty: &[u8] = b"";
        let opening_line_text = lines.get(opening_line_num - 1).unwrap_or(&empty);
        let opening_trimmed = std::str::from_utf8(opening_line_text)
            .unwrap_or("")
            .trim();

        let closing_line_text = &bytes[closing_loc.start_offset()..closing_loc.end_offset()];
        let closing_trimmed = std::str::from_utf8(closing_line_text)
            .unwrap_or("")
            .trim();

        // Find the start of the actual delimiter text (skip leading whitespace)
        let close_content_offset = closing_loc.start_offset()
            + closing_line_text
                .iter()
                .take_while(|&&b| b == b' ' || b == b'\t')
                .count();
        let (close_line, close_col) = self.source.offset_to_line_col(close_content_offset);

        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            close_line,
            close_col,
            format!(
                "`{}` is not aligned with `{}`.",
                closing_trimmed, opening_trimmed
            ),
        ));
    }
}

impl<'pr> Visit<'pr> for HeredocVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let indent = line_indent(self.source, node.location().start_offset());
        self.call_indent_stack.push(indent);
        ruby_prism::visit_call_node(self, node);
        self.call_indent_stack.pop();
    }

    fn visit_string_node(&mut self, node: &ruby_prism::StringNode<'pr>) {
        if let (Some(opening), Some(closing)) = (node.opening_loc(), node.closing_loc()) {
            self.check_heredoc(opening, closing);
        }
        ruby_prism::visit_string_node(self, node);
    }

    fn visit_interpolated_string_node(&mut self, node: &ruby_prism::InterpolatedStringNode<'pr>) {
        if let (Some(opening), Some(closing)) = (node.opening_loc(), node.closing_loc()) {
            self.check_heredoc(opening, closing);
        }
        ruby_prism::visit_interpolated_string_node(self, node);
    }
}

/// Get the indentation (leading spaces) of the line containing the given offset.
fn line_indent(source: &SourceFile, offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut line_start = offset;
    while line_start > 0 && bytes[line_start - 1] != b'\n' {
        line_start -= 1;
    }
    let mut indent = 0;
    while line_start + indent < bytes.len() && bytes[line_start + indent] == b' ' {
        indent += 1;
    }
    indent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(
        ClosingHeredocIndentation,
        "cops/layout/closing_heredoc_indentation"
    );

    #[test]
    fn heredoc_as_argument_aligned_to_outermost_call() {
        let source = b"expect($stdout.string)\n  .to eq(<<~RESULT)\n    content here\nRESULT\n";
        let diags = run_cop_full(&ClosingHeredocIndentation, source);
        assert!(
            diags.is_empty(),
            "Expected no offenses for heredoc argument aligned to outermost call, got: {:?}",
            diags,
        );
    }
}
