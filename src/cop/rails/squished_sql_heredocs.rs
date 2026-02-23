use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SquishedSQLHeredocs;

/// Check if heredoc content contains SQL comments (lines starting with --)
/// that would break if squished. We only check for `--` at the start of
/// content lines (after trimming), not inside strings/identifiers.
fn contains_sql_comments(source: &SourceFile, content_start: usize, content_end: usize) -> bool {
    let bytes = source.as_bytes();
    if content_start >= content_end || content_end > bytes.len() {
        return false;
    }
    let content = &bytes[content_start..content_end];
    for line in content.split(|&b| b == b'\n') {
        let trimmed = line.iter().position(|&c| c != b' ' && c != b'\t');
        if let Some(start) = trimmed {
            if line[start..].starts_with(b"--") {
                // Check this isn't inside a string literal or bracket identifier
                // Simple heuristic: if the line starts with --, it's a comment
                return true;
            }
        }
    }
    false
}

impl Cop for SquishedSQLHeredocs {
    fn name(&self) -> &'static str {
        "Rails/SquishedSQLHeredocs"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTERPOLATED_STRING_NODE, STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Check for heredocs with SQL tag that don't have .squish
        // Could be a StringNode or InterpolatedStringNode

        let (opening_loc, closing_loc, node_loc) = if let Some(s) = node.as_string_node() {
            let opening = match s.opening_loc() {
                Some(o) => o,
                None => return,
            };
            let closing = match s.closing_loc() {
                Some(c) => c,
                None => return,
            };
            (opening, closing, node.location())
        } else if let Some(s) = node.as_interpolated_string_node() {
            let opening = match s.opening_loc() {
                Some(o) => o,
                None => return,
            };
            let closing = match s.closing_loc() {
                Some(c) => c,
                None => return,
            };
            (opening, closing, node.location())
        } else {
            return;
        };

        let bytes = source.as_bytes();
        let opening_text = &bytes[opening_loc.start_offset()..opening_loc.end_offset()];

        // Must be a heredoc starting with << or <<- or <<~
        if !opening_text.starts_with(b"<<") {
            return;
        }

        // Extract the tag name, stripping <<, <<-, <<~
        let tag_start = if opening_text.starts_with(b"<<~") || opening_text.starts_with(b"<<-") {
            3
        } else {
            2
        };
        let tag = &opening_text[tag_start..];

        // Must be SQL heredoc
        if tag != b"SQL" {
            return;
        }

        // Check if .squish is already called by looking at parent context
        // In Prism, if the heredoc has `.squish` chained, the opening will be
        // part of a call node. We check if the opening text contains .squish
        // Actually we need to check if the heredoc opening line has `.squish`
        let opening_line_end = bytes[opening_loc.end_offset()..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| opening_loc.end_offset() + p)
            .unwrap_or(bytes.len());
        let after_opening = &bytes[opening_loc.end_offset()..opening_line_end];

        // Check if `.squish` appears right after the opening tag
        if after_opening.starts_with(b".squish") {
            return;
        }

        // Also check if the opening text itself contains .squish (e.g., <<~SQL.squish)
        if opening_text.windows(7).any(|w| w == b".squish") {
            return;
        }

        // Check for SQL comments that would break if squished
        let content_start = opening_loc.end_offset();
        let content_end = closing_loc.start_offset();
        if contains_sql_comments(source, content_start, content_end) {
            return;
        }

        let heredoc_style = if opening_text.starts_with(b"<<~") {
            "<<~SQL"
        } else if opening_text.starts_with(b"<<-") {
            "<<-SQL"
        } else {
            "<<SQL"
        };

        let (line, column) = source.offset_to_line_col(opening_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{heredoc_style}.squish` instead of `{heredoc_style}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SquishedSQLHeredocs, "cops/rails/squished_sql_heredocs");
}
