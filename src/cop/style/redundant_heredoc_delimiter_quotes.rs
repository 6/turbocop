use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantHeredocDelimiterQuotes;

impl Cop for RedundantHeredocDelimiterQuotes {
    fn name(&self) -> &'static str {
        "Style/RedundantHeredocDelimiterQuotes"
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
        // Check both StringNode (non-interpolated heredoc) and InterpolatedStringNode (heredoc with interp)
        let opening_loc = if let Some(s) = node.as_string_node() {
            s.opening_loc()
        } else if let Some(s) = node.as_interpolated_string_node() {
            s.opening_loc()
        } else {
            return;
        };

        let opening = match opening_loc {
            Some(loc) => loc,
            None => return,
        };

        let open_bytes = opening.as_slice();
        // Must be a heredoc: starts with <<
        if !open_bytes.starts_with(b"<<") {
            return;
        }

        // Check for quoted delimiter: <<~'EOS', <<-"EOS", <<"EOS", <<'EOS'
        // Skip backquote heredocs: <<~`EOS`
        let rest = &open_bytes[2..];
        // Strip optional ~ or -
        let rest = if rest.starts_with(b"~") || rest.starts_with(b"-") {
            &rest[1..]
        } else {
            rest
        };

        if rest.is_empty() {
            return;
        }

        let quote_char = rest[0];
        if quote_char != b'\'' && quote_char != b'"' {
            return;
        }

        // Extract the delimiter name (between quotes)
        let delim = &rest[1..rest.len() - 1]; // strip quotes

        // If the delimiter contains any non-word character, quotes are required.
        // Unquoted heredoc identifiers must be valid Ruby identifiers (alphanumeric + underscore).
        // This matches RuboCop's /\W/ check on the delimiter.
        if delim.is_empty()
            || delim
                .iter()
                .any(|&b| !b.is_ascii_alphanumeric() && b != b'_')
        {
            return;
        }

        if quote_char == b'\'' || quote_char == b'"' {
            // Single-quoted heredocs suppress interpolation and backslash escapes.
            // The quotes are only redundant if the body doesn't use any interpolation
            // patterns or backslash escapes that would be active in a double-quoted heredoc.
            //
            // Double-quoted heredocs with interpolation in the body: the quotes document
            // intent that interpolation is expected. RuboCop skips these.
            let body_bytes = if let Some(s) = node.as_string_node() {
                s.content_loc().as_slice()
            } else if let Some(s) = node.as_interpolated_string_node() {
                // For interpolated strings, check the raw source between opening and closing
                match (s.opening_loc(), s.closing_loc()) {
                    (Some(open), Some(close)) => {
                        &source.as_bytes()[open.end_offset()..close.start_offset()]
                    }
                    _ => &[] as &[u8],
                }
            } else {
                &[] as &[u8]
            };
            // Check for interpolation patterns: #{, #@, #@@, #$
            if body_bytes
                .windows(2)
                .any(|w| w == b"#{" || w == b"#@" || w == b"#$")
            {
                return;
            }
            // Check for backslash escapes — in single-quoted heredocs, backslashes
            // are literal. Removing quotes would make them escape sequences.
            if quote_char == b'\'' && body_bytes.contains(&b'\\') {
                return;
            }
        }

        // Build the suggested replacement
        let prefix = &open_bytes[..open_bytes.len() - rest.len()];
        let prefix_str = String::from_utf8_lossy(prefix);
        let delim_str = String::from_utf8_lossy(delim);

        let (line, column) = source.offset_to_line_col(opening.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Remove the redundant heredoc delimiter quotes, use `{}{}` instead.",
                prefix_str, delim_str
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantHeredocDelimiterQuotes,
        "cops/style/redundant_heredoc_delimiter_quotes"
    );
}
