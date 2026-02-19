use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct QuotedSymbols;

impl Cop for QuotedSymbols {
    fn name(&self) -> &'static str {
        "Style/QuotedSymbols"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "same_as_string_literals");

        // Check for quoted symbols :'foo' or :"foo"
        let loc = node.location();
        let src_bytes = loc.as_slice();

        if node.as_symbol_node().is_some() {
            // Check if it's a quoted symbol
            if src_bytes.starts_with(b":\"") {
                // Double-quoted symbol
                // Check if it needs interpolation or escape sequences
                if src_bytes.len() > 3 {
                    let inner = &src_bytes[2..src_bytes.len().saturating_sub(1)];
                    let has_interpolation = inner.windows(2).any(|w| w == b"#{");
                    let has_escape = inner.contains(&b'\\');
                    let has_single_quote = inner.contains(&b'\'');

                    if has_interpolation || has_escape {
                        return Vec::new(); // Double quotes needed
                    }

                    let prefer_single = match style {
                        "single_quotes" => true,
                        "same_as_string_literals" => {
                            // Look up the Style/StringLiterals EnforcedStyle (injected by config)
                            let sl_style = config.get_str("StringLiteralsEnforcedStyle", "single_quotes");
                            sl_style != "double_quotes"
                        }
                        "double_quotes" => false,
                        _ => true,
                    };

                    if prefer_single && !has_single_quote {
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Prefer single-quoted symbols when you don't need string interpolation or special symbols.".to_string(),
                        )];
                    }
                }
            } else if src_bytes.starts_with(b":'") {
                // Single-quoted symbol
                if src_bytes.len() > 3 {
                    let inner = &src_bytes[2..src_bytes.len().saturating_sub(1)];
                    let has_double_quote = inner.contains(&b'"');

                    if style == "double_quotes" && !has_double_quote {
                        let (line, column) = source.offset_to_line_col(loc.start_offset());
                        return vec![self.diagnostic(
                            source,
                            line,
                            column,
                            "Prefer double-quoted symbols.".to_string(),
                        )];
                    }
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(QuotedSymbols, "cops/style/quoted_symbols");
}
