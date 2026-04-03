use crate::cop::shared::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Layout/HeredocIndentation: checks heredoc body indentation.
///
/// Fixes found in corpus work:
/// - `<<-...squish` can call `.squish` or `.squish!` on the line after the closing
///   delimiter, so the cop checks both the opening line and the line after the
///   closing delimiter.
/// - `<<~` false negatives clustered in tab-indented test files because
///   `base_indent_level` counted only leading spaces on the opening line. RuboCop's
///   indent calculation counts leading tabs too, so the expected heredoc body indent
///   was too small and bad heredocs were missed.
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
            // Bare heredoc (<<FOO) — treated like <<- for indentation checks
            ' '
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

        // For <<- and bare << heredocs:
        // 1. If body is at column 0 → always flag
        // 2. If the heredoc has .squish/.squish! called on it → flag
        //    (matches RuboCop's heredoc_squish? when ActiveSupportExtensionsEnabled)
        // 3. Otherwise (body is indented, no squish) → no offense
        let indent_type_str = if heredoc_type == ' ' { "<<" } else { "<<-" };

        if body_indent == 0 {
            let (line, col) = source.offset_to_line_col(content_start);
            diagnostics.push(self.diagnostic(
                source,
                line,
                col,
                format!(
                    "Use {} spaces for indentation in a heredoc by using `<<~` instead of `{}`.",
                    indentation_width, indent_type_str,
                ),
            ));
        }

        // Check if the heredoc has .squish/.squish! called on it.
        // This can appear either:
        // 1. After the opening: <<-SQL.squish
        // 2. On a separate line after the closing delimiter:
        //    SQL
        //    .squish
        if is_squish_heredoc(bytes, opening_loc.end_offset())
            || is_squish_after_closing(bytes, closing_loc.end_offset())
        {
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
                        "Use {} spaces for indentation in a heredoc by using `<<~` instead of `{}`.",
                        indentation_width, indent_type_str,
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

/// Check if `.squish` or `.squish!` appears on the line after the closing delimiter.
/// Handles the pattern:
///   SQL
///   .squish
/// Note: Prism's closing_loc for heredocs includes the trailing newline,
/// so closing_end points to the start of the next line.
fn is_squish_after_closing(bytes: &[u8], closing_end: usize) -> bool {
    let mut pos = closing_end;

    // Skip leading whitespace on the line after the closing delimiter
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }

    // Check for .squish or .squish!
    if pos >= bytes.len() {
        return false;
    }
    let rest = &bytes[pos..];
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
        lines[line - 1]
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count()
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
/// Counts both spaces and tabs as indentation characters (matching RuboCop's
/// `indent_level` which uses `\s*`).
fn body_indent_level(body: &[u8]) -> usize {
    let mut min_indent = usize::MAX;
    for line in body.split(|&b| b == b'\n') {
        if line.iter().all(|&b| b == b' ' || b == b'\t' || b == b'\r') {
            continue; // Skip blank lines
        }
        let indent = line
            .iter()
            .take_while(|&&b| b == b' ' || b == b'\t')
            .count();
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
    use crate::testutil::run_cop_full;

    crate::cop_fixture_tests!(HeredocIndentation, "cops/layout/heredoc_indentation");

    #[test]
    fn bare_heredoc_body_at_zero_is_offense() {
        let source = b"x = <<SQL\nSELECT * FROM users\nSQL\n";
        let diags = run_cop_full(&HeredocIndentation, source);
        assert!(
            !diags.is_empty(),
            "Expected offense for bare <<SQL with body at column 0"
        );
        assert!(
            diags[0].message.contains("instead of `<<`"),
            "Expected message to mention <<, got: {}",
            diags[0].message
        );
    }

    #[test]
    fn bare_heredoc_indented_body_no_offense() {
        let source = b"x = <<SQL\n  SELECT * FROM users\nSQL\n";
        let diags = run_cop_full(&HeredocIndentation, source);
        assert!(
            diags.is_empty(),
            "Expected no offense for bare <<SQL with indented body, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn squish_after_closing_delimiter() {
        // .squish on a separate line after the closing SQL delimiter
        let source = b"query = <<-SQL\n  SELECT * FROM users\nSQL\n.squish\n";
        let diags = run_cop_full(&HeredocIndentation, source);
        assert!(
            !diags.is_empty(),
            "Expected offense for <<-SQL with .squish on separate line"
        );
    }

    #[test]
    fn test_is_squish_after_closing() {
        // Prism's closing_loc includes the trailing newline, so closing_end
        // points to the start of the line after the delimiter.
        let bytes = b"SQL\n.squish\n";
        assert!(is_squish_after_closing(bytes, 4)); // "SQL\n" ends at 4

        let bytes2 = b"SQL\n  .squish\n";
        assert!(is_squish_after_closing(bytes2, 4)); // with leading whitespace

        let bytes3 = b"SQL\n.squish!\n";
        assert!(is_squish_after_closing(bytes3, 4)); // .squish!

        let bytes4 = b"SQL\nfoo\n";
        assert!(!is_squish_after_closing(bytes4, 4)); // no .squish

        let bytes5 = b"SQL\n";
        assert!(!is_squish_after_closing(bytes5, 4)); // no next line
    }

    #[test]
    fn dash_heredoc_tab_indented_no_offense() {
        let source = b"x = <<-SQL\n\tSELECT * FROM users\nSQL\n";
        let diags = run_cop_full(&HeredocIndentation, source);
        assert!(
            diags.is_empty(),
            "Expected no offense for <<- with tab-indented body, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn squiggly_heredoc_in_tab_indented_scope_is_offense() {
        let source =
            b"test do\n\tassert_equal error.message, <<~ERROR\n  Type mismatch\n\tERROR\nend\n";
        let diags = run_cop_full(&HeredocIndentation, source);
        assert!(
            !diags.is_empty(),
            "Expected offense for <<~ with a tab-indented opening line and under-indented body"
        );
        assert_eq!(
            diags[0].message,
            "Use 2 spaces for indentation in a heredoc."
        );
    }
}
