use crate::cop::shared::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Rails/SquishedSQLHeredocs: checks that SQL heredocs use `.squish`.
///
/// **Investigation (2026-03-08):** 53 FP caused by `contains_sql_comments`
/// only checking for `--` at the start of trimmed lines. RuboCop strips SQL
/// identifier markers (`"..."`, `'...'`, `[...]`) then checks for `--`
/// ANYWHERE in the remaining content. Inline comments like
/// `WHERE id = 1 -- filter` were missed, causing false positives (we
/// reported an offense on heredocs that RuboCop skips).
///
/// Fix: rewrote `contains_sql_comments` to scan byte-by-byte, skipping
/// over quoted (`'`/`"`) and bracket (`[...]`) sections, then detecting
/// `--` anywhere in the unquoted content. Matches RuboCop's
/// `singleline_comments_present?` / `SQL_IDENTIFIER_MARKERS` logic.
///
/// ## Investigation (2026-03-15)
///
/// **FN root cause (2 FN):** Quoted heredoc tags were not matched. `<<~'SQL'` and `<<-'SQL'`
/// use single-quoted delimiters (common for heredocs that should not interpolate). The tag
/// extraction only matched `SQL` literally but not `'SQL'` or `"SQL"`.
/// Fix: strip surrounding quotes from the tag before comparing.
///
/// **FP root cause (4 FP, 2 repos):**
/// 1. `.squish` chained on the line AFTER the closing heredoc tag (3 FP in ransack):
///    ```ruby
///    query = <<-SQL
///      SELECT ...
///    SQL
///    .squish
///    ```
///    RuboCop's AST check (`node.parent&.send_type? && node.parent.method?(:squish)`)
///    catches this because the heredoc is the receiver regardless of line breaks.
///    nitrocop only checked for `.squish` on the opening tag line.
///    Fix: also check for `.squish` after the closing heredoc tag.
///
/// 2. RuboCop's regex-based SQL comment detection creates phantom `--` (1 FP in discourse):
///    `SQL_IDENTIFIER_MARKERS` regex `.+?` requires 1+ char, so empty quotes `''`
///    are NOT stripped. After stripping adjacent non-empty quotes, bare `-` chars
///    can merge to form `--`, triggering false comment detection. Example:
///    `REPLACE(flair_url, 'fas fa-', ''), ' fa-', '-')` → phantom `--`.
///    nitrocop's byte scanner handled quotes correctly but this diverged from RuboCop.
///    Fix: match RuboCop's behavior — require 1+ char inside quotes to strip.
///
/// ## Investigation (2026-03-15, round 2)
///
/// **FP root cause (3 FP, 3 repos: catarse, discourse, forem):**
/// The byte scanner in `contains_sql_comments` allowed quote matching to cross
/// newline boundaries. RuboCop's regex `('.+?')` uses `.` which does NOT match
/// `\n` in Ruby (unless `/m` flag is set). The scanner's `position(|&c| c == quote)`
/// would find a closing quote on a LATER line, consuming all bytes in between -
/// including real `--` comments. This caused the scanner to miss actual SQL
/// comments, returning `false` instead of `true`, which led to false offense reports.
///
/// **FN root cause (1 FN, blackcandy):**
/// Same cross-line quote bug, opposite effect. The heredoc contained
/// `regexp_replace(s.value, '^--- |\n$', '', 'g')` repeated across multiple lines.
/// Cross-line quote matching consumed a `'` from one line and matched it with a
/// `'` on a later line, leaving `---` (from inside `'^--- |\n$'`) exposed in the
/// stripped output. The scanner then incorrectly detected `--` and skipped the
/// heredoc, causing a false negative.
///
/// Fix: Stop quote/bracket matching at newline boundaries in `contains_sql_comments`,
/// matching RuboCop's regex behavior where `.` excludes `\n`.
pub struct SquishedSQLHeredocs;

/// Find the first occurrence of `target` byte in `slice`, stopping at newline.
/// Returns the offset within `slice` if found on the same line, `None` otherwise.
fn find_closing_same_line(slice: &[u8], target: u8) -> Option<usize> {
    for (offset, &ch) in slice.iter().enumerate() {
        if ch == b'\n' {
            return None;
        }
        if ch == target {
            return Some(offset);
        }
    }
    None
}

/// Check if heredoc content contains SQL single-line comments (`--`).
///
/// Matches RuboCop's approach: strip SQL identifier markers (double-quoted,
/// single-quoted, and bracket-quoted identifiers) then check for `--`
/// anywhere in the remaining content.
///
/// RuboCop uses `gsub(/(".+?")|('.+?')|(\[.+?\])/, '')` which REMOVES matched
/// quoted sections and checks the resulting string for `--`. This means
/// characters from adjacent sections can become adjacent after removal,
/// potentially forming phantom `--`. We replicate this by building a stripped
/// string and then checking for `--`, matching RuboCop's behavior exactly.
///
/// **Critical:** RuboCop's regex `.+?` does NOT match newlines (Ruby's `.`
/// excludes `\n` unless `/m` flag is set). Quote matches must therefore be
/// constrained to a single line. Without this constraint, a `'` on one line
/// can match a `'` on a distant line, swallowing `--` comments (causing FPs)
/// or creating phantom `--` from `'---'` patterns across lines (causing FNs).
fn contains_sql_comments(source: &SourceFile, content_start: usize, content_end: usize) -> bool {
    let bytes = source.as_bytes();
    if content_start >= content_end || content_end > bytes.len() {
        return false;
    }
    let content = &bytes[content_start..content_end];

    // Build a new string with SQL identifier markers removed, matching
    // RuboCop's gsub(/(".+?")|('.+?')|(\[.+?\])/, '')
    let mut stripped = Vec::with_capacity(content.len());
    let mut i = 0;
    while i < content.len() {
        let b = content[i];
        // Try to match single-quoted or double-quoted: '.+?' or ".+?" (lazy, 1+ char inside)
        // Must NOT cross newlines (RuboCop's `.` doesn't match `\n` without /m)
        if (b == b'\'' || b == b'"') && i + 2 < content.len() {
            let quote = b;
            // Find the closing quote on the same line (lazy match: first occurrence
            // after 1+ char, stopping at newline)
            if let Some(offset) = find_closing_same_line(&content[i + 2..], quote) {
                // Skip the entire match: opening quote + content + closing quote
                i = i + 2 + offset + 1;
                continue;
            }
        }
        // Try to match bracket identifier: [.+?] (lazy, 1+ char inside, same-line only)
        if b == b'[' && i + 2 < content.len() {
            if let Some(offset) = find_closing_same_line(&content[i + 2..], b']') {
                i = i + 2 + offset + 1;
                continue;
            }
        }
        stripped.push(b);
        i += 1;
    }

    // Check for -- in the stripped content
    stripped.windows(2).any(|w| w == b"--")
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

        let (opening_loc, closing_loc, _node_loc) = if let Some(s) = node.as_string_node() {
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

        // Strip optional quotes around tag: <<~'SQL' or <<~"SQL" -> SQL
        let tag = match tag {
            [b'\'', rest @ .., b'\''] => rest,
            [b'"', rest @ .., b'"'] => rest,
            other => other,
        };

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

        // Check if .squish is chained on the line after the closing heredoc tag:
        //   <<-SQL
        //     ...
        //   SQL
        //   .squish
        let after_close_start = closing_loc.end_offset();
        if after_close_start < bytes.len() {
            let after_close = &bytes[after_close_start..];
            // Skip whitespace (including newline) to find the next non-whitespace
            let trimmed = after_close
                .iter()
                .position(|&b| !b.is_ascii_whitespace())
                .map(|p| &after_close[p..])
                .unwrap_or(&[]);
            if trimmed.starts_with(b".squish") {
                return;
            }
        }

        // Check for SQL comments that would break if squished
        let content_start = opening_loc.end_offset();
        let content_end = closing_loc.start_offset();
        if contains_sql_comments(source, content_start, content_end) {
            return;
        }

        // Build the heredoc style string for the message, preserving quotes if present
        let heredoc_style = {
            let prefix = if opening_text.starts_with(b"<<~") {
                "<<~"
            } else if opening_text.starts_with(b"<<-") {
                "<<-"
            } else {
                "<<"
            };
            // Check if the tag was originally quoted
            let raw_tag = &opening_text[prefix.len()..];
            if raw_tag.starts_with(b"'") {
                format!("{prefix}'SQL'")
            } else if raw_tag.starts_with(b"\"") {
                format!("{prefix}\"SQL\"")
            } else {
                format!("{prefix}SQL")
            }
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
