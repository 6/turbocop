use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantStringEscape;

/// Escape sequences that are always meaningful in double-quoted strings.
/// This includes \\, \", standard escape letters, octal digits, \x, \u, \c, \C, \M,
/// and literal newline/carriage-return after backslash (line continuation).
/// Note: \' and \# are NOT here — they require context-dependent checks.
const MEANINGFUL_ESCAPES: &[u8] = b"\\\"abefnrstv01234567xucCM\n\r";

impl RedundantStringEscape {
    /// Scan raw string content bytes for redundant escape sequences.
    /// `content` is the raw source bytes between delimiters.
    /// `content_start` is the absolute byte offset of the start of content.
    fn scan_escapes(
        &self,
        source: &SourceFile,
        content: &[u8],
        content_start: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut i = 0;

        while i < content.len() {
            if content[i] == b'\\' && i + 1 < content.len() {
                let escaped = content[i + 1];
                let is_redundant = if escaped.is_ascii_alphanumeric() {
                    // Alphanumeric escapes are never redundant (Ruby could give them
                    // meaning in future versions, and many already have meaning).
                    false
                } else if MEANINGFUL_ESCAPES.contains(&escaped) {
                    false
                } else if escaped == b'#' {
                    // \# is only meaningful when disabling interpolation:
                    // \#{, \#$, \#@
                    if i + 2 < content.len() {
                        let next = content[i + 2];
                        !(next == b'{' || next == b'$' || next == b'@')
                    } else {
                        // \# at end of content — redundant
                        true
                    }
                } else {
                    // Any other non-alphanumeric, non-meaningful escape is redundant
                    // This includes \', \:, \=, \,, etc.
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
            // Must be a double-quoted string (not single-quoted, not %q, etc.)
            if open_bytes != b"\"" {
                return;
            }

            let content_loc = s.content_loc();
            let content = content_loc.as_slice();
            let content_start = content_loc.start_offset();
            self.scan_escapes(source, content, content_start, diagnostics);
        } else if let Some(s) = node.as_interpolated_string_node() {
            let opening_loc = match s.opening_loc() {
                Some(o) => o,
                None => return,
            };

            let open_bytes = opening_loc.as_slice();
            // Must be a double-quoted interpolated string
            if open_bytes != b"\"" {
                return;
            }

            // Scan each string part within the interpolated string.
            // EmbeddedStatements parts (#{...}) are skipped — only string segments.
            for part in s.parts().iter() {
                if let Some(str_part) = part.as_string_node() {
                    let content_loc = str_part.content_loc();
                    let content = content_loc.as_slice();
                    let content_start = content_loc.start_offset();
                    self.scan_escapes(source, content, content_start, diagnostics);
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
