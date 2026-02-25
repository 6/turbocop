use crate::cop::node_type::{INTERPOLATED_STRING_NODE, INTERPOLATED_X_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct HeredocDelimiterCase;

impl Cop for HeredocDelimiterCase {
    fn name(&self) -> &'static str {
        "Naming/HeredocDelimiterCase"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            INTERPOLATED_STRING_NODE,
            STRING_NODE,
            INTERPOLATED_X_STRING_NODE,
        ]
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
        let enforced_style = config.get_str("EnforcedStyle", "uppercase");

        // Extract opening and closing locations based on node type.
        // InterpolatedStringNode / StringNode have Option<Location> for opening/closing.
        // InterpolatedXStringNode has Location (non-optional) for both.
        let (opening_start, opening_end, closing_start) =
            if let Some(interp) = node.as_interpolated_string_node() {
                let open = match interp.opening_loc() {
                    Some(loc) => loc,
                    None => return,
                };
                let close_start = interp.closing_loc().map(|l| l.start_offset());
                (open.start_offset(), open.end_offset(), close_start)
            } else if let Some(s) = node.as_string_node() {
                let open = match s.opening_loc() {
                    Some(loc) => loc,
                    None => return,
                };
                let close_start = s.closing_loc().map(|l| l.start_offset());
                (open.start_offset(), open.end_offset(), close_start)
            } else if let Some(x) = node.as_interpolated_x_string_node() {
                let open = x.opening_loc();
                let close = x.closing_loc();
                (
                    open.start_offset(),
                    open.end_offset(),
                    Some(close.start_offset()),
                )
            } else {
                return;
            };

        let bytes = source.as_bytes();
        let opening = &bytes[opening_start..opening_end];

        // Must be a heredoc (starts with <<)
        if !opening.starts_with(b"<<") {
            return;
        }

        // Extract delimiter name (skip <<, ~, -, and quotes)
        let after_arrows = &opening[2..];
        let after_prefix = if after_arrows.starts_with(b"~") || after_arrows.starts_with(b"-") {
            &after_arrows[1..]
        } else {
            after_arrows
        };

        let delimiter = if after_prefix.starts_with(b"'")
            || after_prefix.starts_with(b"\"")
            || after_prefix.starts_with(b"`")
        {
            let quote = after_prefix[0];
            let end = after_prefix[1..]
                .iter()
                .position(|&b| b == quote)
                .unwrap_or(after_prefix.len() - 1);
            &after_prefix[1..1 + end]
        } else {
            // Unquoted delimiter: take only word characters (alphanumeric + underscore)
            let end = after_prefix
                .iter()
                .position(|b| !b.is_ascii_alphanumeric() && *b != b'_')
                .unwrap_or(after_prefix.len());
            if end == 0 {
                return;
            }
            &after_prefix[..end]
        };

        if delimiter.is_empty() {
            return;
        }

        // Skip delimiters with no alphabetic characters — case checking is meaningless
        // for purely non-alpha delimiters like `.,.,` or `---` or `+`.
        // This matches RuboCop which checks /^\w+$/ before applying case rules.
        if !delimiter.iter().any(|b| b.is_ascii_alphabetic()) {
            return;
        }

        let is_uppercase = delimiter
            .iter()
            .all(|b| b.is_ascii_uppercase() || *b == b'_' || b.is_ascii_digit());
        let is_lowercase = delimiter
            .iter()
            .all(|b| b.is_ascii_lowercase() || *b == b'_' || b.is_ascii_digit());

        let offense = match enforced_style {
            "uppercase" => !is_uppercase,
            "lowercase" => !is_lowercase,
            _ => false,
        };

        if offense {
            // RuboCop reports at the closing delimiter (node.loc.heredoc_end)
            let offset = closing_start.unwrap_or(opening_start + 2);
            let (line, column) = source.offset_to_line_col(offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Use {enforced_style} heredoc delimiters."),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(HeredocDelimiterCase, "cops/naming/heredoc_delimiter_case");
}
