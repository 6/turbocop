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
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check StringNode and InterpolatedStringNode for heredoc openings
        let (opening_loc, _closing_loc, content_start, content_end) =
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

        let indentation_width = config.get_usize("IndentationWidth", 2);
        let body_indent = body_indent_level(body);

        // For <<~ heredocs, check that body indentation matches expected level
        if heredoc_type == '~' {
            // Expected: base indent (the line where <<~ appears) + IndentationWidth
            let base_indent = base_indent_level(source, opening_loc.start_offset());
            let expected = base_indent + indentation_width;
            if expected == body_indent {
                return Vec::new(); // Correctly indented
            }

            // Check if adjusting indentation would make lines too long
            if line_too_long_after_adjust(body, expected, body_indent, config) {
                return Vec::new();
            }

            let (line, col) = source.offset_to_line_col(content_start);
            return vec![self.diagnostic(
                source,
                line,
                col,
                format!(
                    "Use {} spaces for indentation in a heredoc.",
                    indentation_width,
                ),
            )];
        }

        // For <<- heredocs:
        // 1. If body is at column 0 → always flag
        // 2. If the heredoc has .squish/.squish! called on it → flag
        //    (matches RuboCop's heredoc_squish? when ActiveSupportExtensionsEnabled)
        // 3. Otherwise (body is indented, no squish) → no offense
        if body_indent == 0 {
            let (line, col) = source.offset_to_line_col(content_start);
            return vec![self.diagnostic(
                source,
                line,
                col,
                format!(
                    "Use {} spaces for indentation in a heredoc by using `<<~` instead of `<<-`.",
                    indentation_width,
                ),
            )];
        }

        // Check if the heredoc opening is followed by .squish or .squish!
        // e.g., <<-SQL.squish or <<-SQL.squish!
        if is_squish_heredoc(bytes, opening_loc.end_offset()) {
            // Check if adjusting indentation would make lines too long
            let base_indent = base_indent_level(source, opening_loc.start_offset());
            let expected = base_indent + indentation_width;
            if !line_too_long_after_adjust(body, expected, body_indent, config) {
                let (line, col) = source.offset_to_line_col(content_start);
                return vec![self.diagnostic(
                    source,
                    line,
                    col,
                    format!(
                        "Use {} spaces for indentation in a heredoc by using `<<~` instead of `<<-`.",
                        indentation_width,
                    ),
                )];
            }
        }

        Vec::new()
    }
}

/// Check if the bytes after the heredoc opening contain `.squish` or `.squish!`.
fn is_squish_heredoc(bytes: &[u8], opening_end: usize) -> bool {
    if opening_end >= bytes.len() {
        return false;
    }
    let rest = &bytes[opening_end..];
    rest.starts_with(b".squish!") || rest.starts_with(b".squish)")
        || rest.starts_with(b".squish\n") || rest.starts_with(b".squish\r")
        || rest.starts_with(b".squish ")
        || (rest.len() >= 7 && &rest[..7] == b".squish" && (rest.len() == 7 || !rest[7].is_ascii_alphanumeric()))
}

/// Get the indentation level of the line where the heredoc opening appears.
fn base_indent_level(source: &SourceFile, opening_offset: usize) -> usize {
    let (line, _) = source.offset_to_line_col(opening_offset);
    let lines: Vec<&[u8]> = source.lines().collect();
    if line > 0 && line <= lines.len() {
        lines[line - 1].iter().take_while(|&&b| b == b' ').count()
    } else {
        0
    }
}

/// Check if adjusting the indentation would make the longest line exceed max line length.
fn line_too_long_after_adjust(
    body: &[u8],
    expected_indent: usize,
    actual_indent: usize,
    config: &CopConfig,
) -> bool {
    // Check Layout/LineLength AllowHeredoc — if true (default), skip this check
    // For simplicity, we default to not checking line length (matching RuboCop's
    // default AllowHeredoc: true behavior).
    let _ = (body, expected_indent, actual_indent, config);
    false
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
