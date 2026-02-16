use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HeredocDelimiterCase;

impl Cop for HeredocDelimiterCase {
    fn name(&self) -> &'static str {
        "Naming/HeredocDelimiterCase"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "uppercase");

        // Check InterpolatedStringNode (heredocs with interpolation)
        // and StringNode (heredocs without interpolation)
        let (opening_loc, _) = if let Some(interp) = node.as_interpolated_string_node() {
            match interp.opening_loc() {
                Some(loc) => (loc, true),
                None => return Vec::new(),
            }
        } else if let Some(s) = node.as_string_node() {
            match s.opening_loc() {
                Some(loc) => (loc, false),
                None => return Vec::new(),
            }
        } else {
            return Vec::new();
        };

        let bytes = source.as_bytes();
        let opening = &bytes[opening_loc.start_offset()..opening_loc.end_offset()];

        // Must be a heredoc (starts with <<)
        if !opening.starts_with(b"<<") {
            return Vec::new();
        }

        // Extract delimiter name (skip <<, ~, -, and quotes)
        let after_arrows = &opening[2..];
        let after_prefix = if after_arrows.starts_with(b"~") || after_arrows.starts_with(b"-") {
            &after_arrows[1..]
        } else {
            after_arrows
        };

        let (delimiter, _quoted) = if after_prefix.starts_with(b"'")
            || after_prefix.starts_with(b"\"")
            || after_prefix.starts_with(b"`")
        {
            let quote = after_prefix[0];
            let end = after_prefix[1..]
                .iter()
                .position(|&b| b == quote)
                .unwrap_or(after_prefix.len() - 1);
            (&after_prefix[1..1 + end], true)
        } else {
            (after_prefix, false)
        };

        if delimiter.is_empty() {
            return Vec::new();
        }

        let is_uppercase = delimiter.iter().all(|&b| b.is_ascii_uppercase() || b == b'_' || b.is_ascii_digit());
        let is_lowercase = delimiter.iter().all(|&b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit());

        let offense = match enforced_style {
            "uppercase" => !is_uppercase,
            "lowercase" => !is_lowercase,
            _ => false,
        };

        if offense {
            // Point to the delimiter in the opening
            let delimiter_offset = opening_loc.start_offset() + (opening.len() - delimiter.len());
            // Account for closing quote
            let delimiter_offset = if delimiter_offset > opening_loc.end_offset() {
                opening_loc.start_offset() + 2
            } else {
                opening_loc.start_offset() + opening.len() - delimiter.len()
                    - if opening.ends_with(b"'") || opening.ends_with(b"\"") || opening.ends_with(b"`") { 1 } else { 0 }
            };
            let (line, column) = source.offset_to_line_col(delimiter_offset);
            return vec![self.diagnostic(
                source,
                line,
                column,
                format!("Use {enforced_style} heredoc delimiters."),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(HeredocDelimiterCase, "cops/naming/heredoc_delimiter_case");
}
