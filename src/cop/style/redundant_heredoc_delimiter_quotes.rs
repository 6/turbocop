use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantHeredocDelimiterQuotes;

impl Cop for RedundantHeredocDelimiterQuotes {
    fn name(&self) -> &'static str {
        "Style/RedundantHeredocDelimiterQuotes"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check both StringNode (non-interpolated heredoc) and InterpolatedStringNode (heredoc with interp)
        let opening_loc = if let Some(s) = node.as_string_node() {
            s.opening_loc()
        } else if let Some(s) = node.as_interpolated_string_node() {
            s.opening_loc()
        } else {
            return Vec::new();
        };

        let opening = match opening_loc {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let open_bytes = opening.as_slice();
        // Must be a heredoc: starts with <<
        if !open_bytes.starts_with(b"<<") {
            return Vec::new();
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
            return Vec::new();
        }

        let quote_char = rest[0];
        if quote_char != b'\'' && quote_char != b'"' {
            return Vec::new();
        }

        // If it's single-quoted, it's always redundant if the content has no interpolation
        // (single-quoted heredocs disable interpolation, but if the content doesn't use
        // interpolation syntax, the quotes are redundant)
        // If it's double-quoted, it's redundant because double-quote is the default
        // However, single-quoted heredocs with interpolation-like content (#{ }) need the quotes.
        // Let's check: for single-quoted, scan content for interpolation patterns
        if quote_char == b'\'' {
            // Check if the heredoc body contains interpolation-like patterns
            let body_bytes = &source.as_bytes()[opening.end_offset()..node.location().end_offset()];
            if body_bytes.windows(2).any(|w| w == b"#{" || w == b"#@" || w == b"#$") {
                return Vec::new();
            }
        }

        // Build the suggested replacement
        let prefix = &open_bytes[..open_bytes.len() - rest.len()];
        let delim = &rest[1..rest.len() - 1]; // strip quotes
        let prefix_str = String::from_utf8_lossy(prefix);
        let delim_str = String::from_utf8_lossy(delim);

        let (line, column) = source.offset_to_line_col(opening.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            format!(
                "Remove the redundant heredoc delimiter quotes, use `{}{}` instead.",
                prefix_str, delim_str
            ),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantHeredocDelimiterQuotes, "cops/style/redundant_heredoc_delimiter_quotes");
}
