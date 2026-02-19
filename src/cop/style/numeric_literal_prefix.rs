use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::INTEGER_NODE;

pub struct NumericLiteralPrefix;

impl Cop for NumericLiteralPrefix {
    fn name(&self) -> &'static str {
        "Style/NumericLiteralPrefix"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTEGER_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let int_node = match node.as_integer_node() {
            Some(i) => i,
            None => return Vec::new(),
        };

        let loc = int_node.location();
        let src = loc.as_slice();
        let src_str = match std::str::from_utf8(src) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        // Strip any underscores for prefix matching
        let clean = src_str.replace('_', "");

        let enforced_octal_style = config.get_str("EnforcedOctalStyle", "zero_with_o");

        let (line, column) = source.offset_to_line_col(loc.start_offset());

        // Check uppercase hex prefix: 0X...
        if clean.starts_with("0X") {
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use 0x for hexadecimal literals.".to_string(),
            )];
        }

        // Check uppercase binary prefix: 0B...
        if clean.starts_with("0B") {
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Use 0b for binary literals.".to_string(),
            )];
        }

        // Check decimal prefix: 0d... or 0D...
        if clean.starts_with("0d") || clean.starts_with("0D") {
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Do not use prefixes for decimal literals.".to_string(),
            )];
        }

        // Octal handling
        if enforced_octal_style == "zero_with_o" {
            // Bad: 0O... (uppercase)
            if clean.starts_with("0O") {
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use 0o for octal literals.".to_string(),
                )];
            }
            // Bad: plain 0... without 'o' (e.g., 01234)
            // Must be octal: starts with 0, followed by digits 0-7, not 0x/0b/0d/0o
            if clean.len() > 1
                && clean.starts_with('0')
                && !clean.starts_with("0x")
                && !clean.starts_with("0b")
                && !clean.starts_with("0o")
                && clean[1..].bytes().all(|b| b.is_ascii_digit() && b < b'8')
            {
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use 0o for octal literals.".to_string(),
                )];
            }
        } else if enforced_octal_style == "zero_only" {
            // Bad: 0o... or 0O...
            if clean.starts_with("0o") || clean.starts_with("0O") {
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Use 0 for octal literals.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NumericLiteralPrefix, "cops/style/numeric_literal_prefix");
}
