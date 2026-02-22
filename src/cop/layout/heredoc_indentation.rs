use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HeredocIndentation;

impl Cop for HeredocIndentation {
    fn name(&self) -> &'static str {
        "Layout/HeredocIndentation"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTERPOLATED_STRING_NODE, STRING_NODE]
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
        // Check StringNode and InterpolatedStringNode for heredoc openings.
        let (opening_loc, closing_loc, raw_content_start) = if let Some(s) = node.as_string_node() {
            match (s.opening_loc(), s.closing_loc()) {
                (Some(o), Some(c)) => (o, c, Some(s.content_loc().start_offset())),
                _ => return,
            }
        } else if let Some(s) = node.as_interpolated_string_node() {
            match (s.opening_loc(), s.closing_loc()) {
                (Some(o), Some(c)) => (o, c, None),
                _ => return,
            }
        } else {
            return;
        };

        let src_bytes = source.as_bytes();

        // Content ends at the start of the closing delimiter's line
        let mut content_end = closing_loc.start_offset();
        while content_end > 0 && src_bytes[content_end - 1] != b'\n' {
            content_end -= 1;
        }

        // Find content_start: for StringNode, content_loc().start_offset()
        // is usually correct but for <<~ heredocs, Prism may report the same
        // offset for all heredocs on the same line. For InterpolatedStringNode,
        // use the first part's location.
        //
        // We use the closing_loc to bound the body: walk backwards from
        // content_end to get the real body start, using the line after the
        // opening (or after the previous heredoc) as a floor.
        let content_start = if let Some(s) = node.as_interpolated_string_node() {
            let parts = s.parts();
            if parts.is_empty() {
                return;
            }
            // For InterpolatedStringNode, use the first part's location
            let first_part_start = parts.iter().next().unwrap().location().start_offset();
            let mut line_start = first_part_start;
            while line_start > 0 && src_bytes[line_start - 1] != b'\n' {
                line_start -= 1;
            }
            line_start
        } else if let Some(rcs) = raw_content_start {
            // For StringNode, content_loc might be unreliable for <<~ with
            // multiple heredocs. Verify: the content_start should be between
            // the line after the opening and content_end.
            let mut line_start = rcs;
            while line_start > 0 && src_bytes[line_start - 1] != b'\n' {
                line_start -= 1;
            }
            // Verify this is within range
            if line_start < content_end {
                line_start
            } else {
                // Fallback: scan forward from opening
                let mut start = opening_loc.end_offset();
                while start < src_bytes.len() && src_bytes[start] != b'\n' {
                    start += 1;
                }
                if start < src_bytes.len() {
                    start + 1
                } else {
                    start
                }
            }
        } else {
            // Fallback: scan forward from opening
            let mut start = opening_loc.end_offset();
            while start < src_bytes.len() && src_bytes[start] != b'\n' {
                start += 1;
            }
            if start < src_bytes.len() {
                start + 1
            } else {
                start
            }
        };

        if content_start >= content_end {
            return;
        }

        let bytes = source.as_bytes();
        let opening = &bytes[opening_loc.start_offset()..opening_loc.end_offset()];

        // Must be a heredoc
        if !opening.starts_with(b"<<") {
            return;
        }

        // Determine heredoc type
        let after_arrows = &opening[2..];
        let heredoc_type = if after_arrows.starts_with(b"~") {
            '~'
        } else if after_arrows.starts_with(b"-") {
            '-'
        } else {
            // Simple heredoc (<<FOO) — no indentation control expected
            return;
        };

        // Get heredoc body content
        let body = &bytes[content_start..content_end];
        if body
            .iter()
            .all(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
        {
            return; // Empty body
        }

        let indentation_width = config.get_usize("IndentationWidth", 2);
        let body_indent = body_indent_level(body);

        // For <<~ heredocs, check that body indentation matches expected level
        if heredoc_type == '~' {
            // Expected: base indent (the line where <<~ appears) + IndentationWidth
            let base_indent = base_indent_level(source, opening_loc.start_offset());
            let expected = base_indent + indentation_width;
            if expected == body_indent {
                return; // Correctly indented
            }

            // Check if adjusting indentation would make lines too long
            if line_too_long_after_adjust(body, expected, body_indent, config) {
                return;
            }

            let (line, col) = source.offset_to_line_col(content_start);
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                format!(
                    "Use {} spaces for indentation in a heredoc.",
                    indentation_width,
                ),
            ));
        }

        // For <<- heredocs:
        // 1. If body is at column 0 → always flag
        // 2. If the heredoc has .squish/.squish! called on it → flag
        //    (matches RuboCop's heredoc_squish? when ActiveSupportExtensionsEnabled)
        // 3. Otherwise (body is indented, no squish) → no offense
        if body_indent == 0 {
            let (line, col) = source.offset_to_line_col(content_start);
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                format!(
                    "Use {} spaces for indentation in a heredoc by using `<<~` instead of `<<-`.",
                    indentation_width,
                ),
            ));
        }

        // Check if the heredoc opening is followed by .squish or .squish!
        // e.g., <<-SQL.squish or <<-SQL.squish!
        if is_squish_heredoc(bytes, opening_loc.end_offset()) {
            // Check if adjusting indentation would make lines too long
            let base_indent = base_indent_level(source, opening_loc.start_offset());
            let expected = base_indent + indentation_width;
            if !line_too_long_after_adjust(body, expected, body_indent, config) {
                let (line, col) = source.offset_to_line_col(content_start);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    col,
                    format!(
                        "Use {} spaces for indentation in a heredoc by using `<<~` instead of `<<-`.",
                        indentation_width,
                    ),
                ));
            }
        }
    }
}

/// Check if the bytes after the heredoc opening contain `.squish` or `.squish!`.
fn is_squish_heredoc(bytes: &[u8], opening_end: usize) -> bool {
    if opening_end >= bytes.len() {
        return false;
    }
    let rest = &bytes[opening_end..];
    rest.starts_with(b".squish!")
        || rest.starts_with(b".squish)")
        || rest.starts_with(b".squish\n")
        || rest.starts_with(b".squish\r")
        || rest.starts_with(b".squish ")
        || (rest.len() >= 7
            && &rest[..7] == b".squish"
            && (rest.len() == 7 || !rest[7].is_ascii_alphanumeric()))
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
