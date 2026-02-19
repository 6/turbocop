use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};

pub struct HeredocDelimiterNaming;

// Default forbidden patterns: EO followed by one uppercase letter, or END
fn is_forbidden_delimiter(delimiter: &str) -> bool {
    // Default: /(^|\s)(EO[A-Z]{1}|END)(\s|$)/i
    let d = delimiter.to_uppercase();
    if d == "END" {
        return true;
    }
    if d.len() == 3 && d.starts_with("EO") && d.as_bytes()[2].is_ascii_alphabetic() {
        return true;
    }
    false
}

impl Cop for HeredocDelimiterNaming {
    fn name(&self) -> &'static str {
        "Naming/HeredocDelimiterNaming"
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
    ) -> Vec<Diagnostic> {
        let _forbidden_delimiters = config.get_string_array("ForbiddenDelimiters");

        // Check InterpolatedStringNode and StringNode for heredoc openings
        let opening_loc = if let Some(interp) = node.as_interpolated_string_node() {
            interp.opening_loc()
        } else if let Some(s) = node.as_string_node() {
            s.opening_loc()
        } else {
            return Vec::new();
        };

        let opening_loc = match opening_loc {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let bytes = source.as_bytes();
        let opening = &bytes[opening_loc.start_offset()..opening_loc.end_offset()];

        if !opening.starts_with(b"<<") {
            return Vec::new();
        }

        // Extract delimiter
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
            after_prefix
        };

        let delimiter_str = std::str::from_utf8(delimiter).unwrap_or("");
        if delimiter_str.is_empty() {
            return Vec::new();
        }

        if is_forbidden_delimiter(delimiter_str) {
            let delimiter_offset = opening_loc.start_offset() + opening.len() - delimiter.len()
                - if opening.ends_with(b"'") || opening.ends_with(b"\"") || opening.ends_with(b"`") { 1 } else { 0 };
            let (line, column) = source.offset_to_line_col(delimiter_offset);
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use meaningful heredoc delimiters.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        HeredocDelimiterNaming,
        "cops/naming/heredoc_delimiter_naming"
    );
}
