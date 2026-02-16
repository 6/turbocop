use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SymbolLiteral;

impl Cop for SymbolLiteral {
    fn name(&self) -> &'static str {
        "Style/SymbolLiteral"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let sym_node = match node.as_symbol_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Check if the symbol uses string syntax: :"foo"
        let opening_loc = match sym_node.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let opening = opening_loc.as_slice();
        // Must start with :" (quoted symbol)
        if opening != b":\"" && opening != b":'" {
            return Vec::new();
        }

        // Check if the content is a simple word (only word chars: alphanumeric + underscore)
        let content_loc = match sym_node.value_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };
        let content = content_loc.as_slice();
        if content.is_empty() {
            return Vec::new();
        }

        // First char must not be a digit
        if content[0].is_ascii_digit() {
            return Vec::new();
        }

        // All chars must be word characters
        let all_word_chars = content.iter().all(|&b: &u8| b.is_ascii_alphanumeric() || b == b'_');
        if !all_word_chars {
            return Vec::new();
        }

        let loc = sym_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not use strings for word-like symbol literals.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SymbolLiteral, "cops/style/symbol_literal");
}
