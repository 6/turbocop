use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ClosingHeredocIndentation;

impl Cop for ClosingHeredocIndentation {
    fn name(&self) -> &'static str {
        "Layout/ClosingHeredocIndentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Handle both StringNode (non-interpolated heredoc) and InterpolatedStringNode
        let (opening_loc, closing_loc) =
            if let Some(s) = node.as_string_node() {
                match (s.opening_loc(), s.closing_loc()) {
                    (Some(o), Some(c)) => (o, c),
                    _ => return Vec::new(),
                }
            } else if let Some(s) = node.as_interpolated_string_node() {
                match (s.opening_loc(), s.closing_loc()) {
                    (Some(o), Some(c)) => (o, c),
                    _ => return Vec::new(),
                }
            } else {
                return Vec::new();
            };

        let bytes = source.as_bytes();
        let opening = &bytes[opening_loc.start_offset()..opening_loc.end_offset()];

        // Must be a heredoc
        if !opening.starts_with(b"<<") {
            return Vec::new();
        }

        // Skip simple heredocs (<<FOO without - or ~) since they have no indentation control
        let after_arrows = &opening[2..];
        if !after_arrows.starts_with(b"~") && !after_arrows.starts_with(b"-") {
            return Vec::new();
        }

        // Get indentation of the opening line
        let opening_line_indent = line_indent(source, opening_loc.start_offset());

        // Get indentation of the closing line
        let closing_line_indent = line_indent(source, closing_loc.start_offset());

        if opening_line_indent == closing_line_indent {
            return Vec::new();
        }

        // Get the opening and closing text for the message
        let (opening_line_num, _) = source.offset_to_line_col(opening_loc.start_offset());
        let lines: Vec<&[u8]> = source.lines().collect();
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
        let (close_line, close_col) = source.offset_to_line_col(close_content_offset);

        vec![self.diagnostic(
            source,
            close_line,
            close_col,
            format!(
                "`{}` is not aligned with `{}`.",
                closing_trimmed, opening_trimmed
            ),
        )]
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

    crate::cop_fixture_tests!(
        ClosingHeredocIndentation,
        "cops/layout/closing_heredoc_indentation"
    );
}
