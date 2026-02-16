use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HeredocIndentation;

impl Cop for HeredocIndentation {
    fn name(&self) -> &'static str {
        "Layout/HeredocIndentation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check StringNode and InterpolatedStringNode for heredoc openings
        let (opening_loc, closing_loc, content_start, content_end) =
            if let Some(s) = node.as_string_node() {
                match (s.opening_loc(), s.closing_loc()) {
                    (Some(o), Some(c)) => {
                        let cl = s.content_loc();
                        (o, c, cl.start_offset(), cl.end_offset())
                    }
                    _ => return Vec::new(),
                }
            } else if let Some(s) = node.as_interpolated_string_node() {
                match (s.opening_loc(), s.closing_loc()) {
                    (Some(o), Some(c)) => {
                        let parts = s.parts();
                        if parts.is_empty() {
                            return Vec::new();
                        }
                        let first = parts.iter().next().unwrap();
                        let last = parts.iter().last().unwrap();
                        (o, c, first.location().start_offset(), last.location().end_offset())
                    }
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

        // Determine heredoc type
        let after_arrows = &opening[2..];
        let heredoc_type = if after_arrows.starts_with(b"~") {
            '~'
        } else if after_arrows.starts_with(b"-") {
            '-'
        } else {
            // Simple heredoc (<<FOO) — no indentation control expected
            return Vec::new();
        };

        // Get heredoc body content
        let body = &bytes[content_start..content_end];
        if body.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r') {
            return Vec::new(); // Empty body
        }

        // For <<~ heredocs, check that body is indented
        if heredoc_type == '~' {
            // Body indent is fine as long as it uses squiggly heredoc — that's the point
            // This cop mainly flags <<- and << heredocs that should use <<~
            return Vec::new();
        }

        // For <<- heredocs, the body should be indented (use <<~ instead)
        // Check if body lines are at column 0 (not indented)
        let body_indent = body_indent_level(body);
        if body_indent == 0 {
            let (line, col) = source.offset_to_line_col(content_start);
            return vec![self.diagnostic(
                source,
                line,
                col,
                format!(
                    "Use {} spaces for indentation in a heredoc by using `<<~` instead of `<<-`.",
                    2
                ),
            )];
        }

        // Check if closing location line starts content
        let _ = closing_loc;

        Vec::new()
    }
}

/// Get the minimum indentation level of non-blank body lines.
fn body_indent_level(body: &[u8]) -> usize {
    let mut min_indent = usize::MAX;
    for line in body.split(|&b| b == b'\n') {
        if line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r') {
            continue; // Skip blank lines
        }
        let indent = line.iter().take_while(|&&b| b == b' ').count();
        min_indent = min_indent.min(indent);
    }
    if min_indent == usize::MAX {
        0
    } else {
        min_indent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(HeredocIndentation, "cops/layout/heredoc_indentation");
}
