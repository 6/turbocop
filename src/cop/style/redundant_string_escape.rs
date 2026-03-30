use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Checks for redundant escape sequences in string literals.
///
/// Handles double-quoted strings (`"..."`), interpolating heredocs
/// (`<<~STR`, `<<-STR`, `<<"STR"`), and interpolating percent literals
/// (`%(...)`, `%Q(...)`, `%Q!...!`).
///
/// FP fixes applied:
/// - `#\{`, `#\$`, `#\@` patterns are not flagged (they disable interpolation).
/// - `\#\{` pattern: `\#` is not flagged (it pairs with `\{` to disable
///   interpolation), but `\{` IS flagged as redundant since `\#` suffices.
/// - Non-ASCII bytes after `\` are not flagged (Unicode alphanumeric chars
///   like `\ê` match RuboCop's `[[:alnum:]]` exemption).
///
/// FN fixes applied:
/// - Heredocs (non-single-quoted) are now scanned for redundant escapes.
///   `\"` and `\'` are redundant in heredocs since `"` and `'` are not
///   delimiters. `\ ` (backslash-space) is allowed in heredocs.
/// - `%(...)` and `%Q(...)` percent literals are now scanned. `\"` is
///   redundant since `"` is not the delimiter; `\)` is allowed since `)`
///   is the closing delimiter.
pub struct RedundantStringEscape;

/// Escape sequences that are always meaningful in double-quoted-style strings.
/// This includes \\, standard escape letters, octal digits, \x, \u, \c, \C, \M,
/// and literal newline/carriage-return after backslash (line continuation).
/// Note: \", \', and \# are NOT here — they require context-dependent checks.
const MEANINGFUL_ESCAPES: &[u8] = b"\\abefnrstv01234567xucCM\n\r";

/// Returns the matching closing bracket for an opening bracket byte,
/// or the same byte for symmetric delimiters.
fn matching_bracket(open: u8) -> u8 {
    match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        other => other,
    }
}

/// Analyze the opening delimiter to determine if the string supports
/// escape processing. Returns `(delimiter_chars, is_heredoc)` or None
/// if the string type should not be processed (single-quoted, %q, etc.).
fn analyze_opening(open_bytes: &[u8]) -> Option<(Vec<u8>, bool)> {
    // Standard double-quoted string
    if open_bytes == b"\"" {
        return Some((vec![b'"'], false));
    }

    // Heredocs: <<FOO, <<-FOO, <<~FOO, <<"FOO", <<-"FOO", <<~"FOO"
    // Skip single-quoted heredocs: <<'FOO', <<-'FOO', <<~'FOO'
    if open_bytes.starts_with(b"<<") {
        if open_bytes.contains(&b'\'') {
            return None;
        }
        return Some((vec![], true));
    }

    // Percent literals: %(foo), %Q(foo), %Q!foo!, etc.
    // Skip non-interpolating: %q, %w, %i
    if open_bytes.starts_with(b"%") && open_bytes.len() >= 2 {
        let second = open_bytes[1];
        if second == b'q' || second == b'w' || second == b'i' {
            return None;
        }
        let last = *open_bytes.last()?;
        let closing = matching_bracket(last);
        let mut delimiters = vec![last];
        if closing != last {
            delimiters.push(closing);
        }
        return Some((delimiters, false));
    }

    None
}

impl RedundantStringEscape {
    /// Scan raw string content bytes for redundant escape sequences.
    /// `content` is the raw source bytes between delimiters.
    /// `content_start` is the absolute byte offset of the start of content.
    /// `delimiter_chars` contains the chars that are valid to escape (delimiters).
    /// `is_heredoc` indicates if this is a heredoc string (affects `\ ` handling).
    fn scan_escapes(
        &self,
        source: &SourceFile,
        content: &[u8],
        content_start: usize,
        delimiter_chars: &[u8],
        is_heredoc: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut i = 0;

        while i < content.len() {
            if content[i] == b'\\' && i + 1 < content.len() {
                let escaped = content[i + 1];
                let is_redundant = if MEANINGFUL_ESCAPES.contains(&escaped) {
                    false
                } else if escaped.is_ascii_alphanumeric() {
                    // Alphanumeric escapes are never redundant (Ruby could give them
                    // meaning in future versions, and many already have meaning).
                    false
                } else if !escaped.is_ascii() {
                    // Non-ASCII bytes (part of multi-byte UTF-8 chars like ê)
                    // are not flagged, matching RuboCop's [[:alnum:]] exemption.
                    false
                } else if delimiter_chars.contains(&escaped) {
                    // Escaping the delimiter character is meaningful
                    false
                } else if escaped == b'#' {
                    // \# is only meaningful when disabling interpolation:
                    // \#{, \#$, \#@
                    if i + 2 < content.len() {
                        let next = content[i + 2];
                        if next == b'{' || next == b'$' || next == b'@' {
                            // \#{, \#$, \#@ — disabling interpolation
                            false
                        } else if next == b'\\' && i + 3 < content.len() && content[i + 3] == b'{' {
                            // \#\{ — \# is not redundant (pairs with \{ to disable interp)
                            false
                        } else {
                            true
                        }
                    } else {
                        // \# at end of content — redundant
                        true
                    }
                } else if escaped == b'{' || escaped == b'$' || escaped == b'@' {
                    // Check if preceded by '#' (not '\#') — disabling interpolation
                    // Patterns: #\{, #\$, #\@
                    if i > 0 && content[i - 1] == b'#' {
                        // Count consecutive backslashes before the '#'
                        let hash_pos = i - 1;
                        let mut bs_count: usize = 0;
                        let mut p = hash_pos;
                        while p > 0 {
                            p -= 1;
                            if content[p] == b'\\' {
                                bs_count += 1;
                            } else {
                                break;
                            }
                        }
                        // Even backslashes (including 0): '#' is literal → not redundant
                        // Odd backslashes: '#' is escaped (\#\{) → \{ is redundant
                        bs_count % 2 != 0
                    } else {
                        true
                    }
                } else if escaped == b' ' && is_heredoc {
                    // \space is not redundant in heredocs
                    false
                } else {
                    // Any other non-alphanumeric, non-meaningful escape is redundant
                    true
                };

                if is_redundant {
                    let abs_offset = content_start + i;
                    let (line, column) = source.offset_to_line_col(abs_offset);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Redundant escape of `{}` in string.", escaped as char),
                    ));
                }
                i += 2;
            } else {
                i += 1;
            }
        }
    }
}

impl Cop for RedundantStringEscape {
    fn name(&self) -> &'static str {
        "Style/RedundantStringEscape"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE, INTERPOLATED_STRING_NODE]
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
        if let Some(s) = node.as_string_node() {
            let opening_loc = match s.opening_loc() {
                Some(o) => o,
                None => return,
            };

            let open_bytes = opening_loc.as_slice();
            let (delimiter_chars, is_heredoc) = match analyze_opening(open_bytes) {
                Some(ctx) => ctx,
                None => return,
            };

            let content_loc = s.content_loc();
            let content = content_loc.as_slice();
            let content_start = content_loc.start_offset();
            self.scan_escapes(source, content, content_start, &delimiter_chars, is_heredoc, diagnostics);
        } else if let Some(s) = node.as_interpolated_string_node() {
            let opening_loc = match s.opening_loc() {
                Some(o) => o,
                None => return,
            };

            let open_bytes = opening_loc.as_slice();
            let (delimiter_chars, is_heredoc) = match analyze_opening(open_bytes) {
                Some(ctx) => ctx,
                None => return,
            };

            // Scan each string part within the interpolated string.
            // EmbeddedStatements parts (#{...}) are skipped — only string segments.
            for part in s.parts().iter() {
                if let Some(str_part) = part.as_string_node() {
                    let content_loc = str_part.content_loc();
                    let content = content_loc.as_slice();
                    let content_start = content_loc.start_offset();
                    self.scan_escapes(source, content, content_start, &delimiter_chars, is_heredoc, diagnostics);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantStringEscape, "cops/style/redundant_string_escape");
}
