use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::SYMBOL_NODE;

pub struct SymbolLiteral;

impl Cop for SymbolLiteral {
    fn name(&self) -> &'static str {
        "Style/SymbolLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[SYMBOL_NODE]
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
        let sym_node = match node.as_symbol_node() {
            Some(s) => s,
            None => return,
        };

        // Check if the symbol uses string syntax: :"foo"
        let opening_loc = match sym_node.opening_loc() {
            Some(loc) => loc,
            None => return,
        };

        let opening = opening_loc.as_slice();
        // Must start with :" (quoted symbol)
        if opening != b":\"" && opening != b":'" {
            return;
        }

        // Check if the content is a simple word (only word chars: alphanumeric + underscore)
        let content_loc = match sym_node.value_loc() {
            Some(loc) => loc,
            None => return,
        };
        let content = content_loc.as_slice();
        if content.is_empty() {
            return;
        }

        // First char must not be a digit
        if content[0].is_ascii_digit() {
            return;
        }

        // All chars must be word characters
        let all_word_chars = content.iter().all(|&b: &u8| b.is_ascii_alphanumeric() || b == b'_');
        if !all_word_chars {
            return;
        }

        let loc = sym_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not use strings for word-like symbol literals.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SymbolLiteral, "cops/style/symbol_literal");
}
