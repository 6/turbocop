use crate::cop::node_type::SYMBOL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct QuotedSymbols;

impl Cop for QuotedSymbols {
    fn name(&self) -> &'static str {
        "Style/QuotedSymbols"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[SYMBOL_NODE]
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
        let style = config.get_str("EnforcedStyle", "same_as_string_literals");

        let sym = match node.as_symbol_node() {
            Some(s) => s,
            None => return,
        };

        let loc = sym.location();
        let src_bytes = loc.as_slice();

        // Determine if this is a hash-key symbol (e.g. "invest": or 'invest':)
        // vs a standalone symbol (e.g. :"foo" or :'foo')
        let is_hash_key_double = src_bytes.starts_with(b"\"") && src_bytes.ends_with(b"\":");
        let is_hash_key_single = src_bytes.starts_with(b"'") && src_bytes.ends_with(b"':");
        let is_standalone_double = src_bytes.starts_with(b":\"");
        let is_standalone_single = src_bytes.starts_with(b":'");

        let is_double_quoted = is_hash_key_double || is_standalone_double;
        let is_single_quoted = is_hash_key_single || is_standalone_single;

        if is_double_quoted {
            // Extract inner content (between the quotes)
            let inner = if is_hash_key_double {
                &src_bytes[1..src_bytes.len().saturating_sub(2)] // strip leading " and trailing ":
            } else {
                &src_bytes[2..src_bytes.len().saturating_sub(1)] // strip leading :" and trailing "
            };
            if inner.is_empty() {
                return;
            }

            let has_interpolation = inner.windows(2).any(|w| w == b"#{");
            let has_escape = inner.contains(&b'\\');
            let has_single_quote = inner.contains(&b'\'');

            if has_interpolation || has_escape {
                return; // Double quotes needed
            }

            let prefer_single = match style {
                "single_quotes" => true,
                "same_as_string_literals" => {
                    let sl_style = config.get_str("StringLiteralsEnforcedStyle", "single_quotes");
                    sl_style != "double_quotes"
                }
                "double_quotes" => false,
                _ => true,
            };

            if prefer_single && !has_single_quote {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer single-quoted symbols when you don't need string interpolation or special symbols.".to_string(),
                ));
            }
        } else if is_single_quoted {
            let inner = if is_hash_key_single {
                &src_bytes[1..src_bytes.len().saturating_sub(2)] // strip leading ' and trailing ':
            } else {
                &src_bytes[2..src_bytes.len().saturating_sub(1)] // strip leading :' and trailing '
            };
            if inner.is_empty() {
                return;
            }

            let has_double_quote = inner.contains(&b'"');

            if style == "double_quotes" && !has_double_quote {
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Prefer double-quoted symbols.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(QuotedSymbols, "cops/style/quoted_symbols");
}
