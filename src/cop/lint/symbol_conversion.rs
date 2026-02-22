use crate::cop::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SymbolConversion;

/// Check if a symbol value can be used as a bare symbol (no quotes needed).
/// Returns true for identifiers like `invest`, `foo_bar`, etc.
fn can_be_bare_symbol(value: &[u8]) -> bool {
    if value.is_empty() {
        return false;
    }
    value
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_')
        && !value[0].is_ascii_digit()
}

impl Cop for SymbolConversion {
    fn name(&self) -> &'static str {
        "Lint/SymbolConversion"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE, SYMBOL_NODE]
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
        let _style = config.get_str("EnforcedStyle", "strict");

        // Check SymbolNode: quoted hash key like { 'invest': val } or { "invest": val }
        if let Some(sym) = node.as_symbol_node() {
            let src = sym.location().as_slice();
            // Quoted hash key: source starts with ' or " and ends with ': or ":
            // e.g. 'invest': or "invest":
            if (src.starts_with(b"'") || src.starts_with(b"\""))
                && (src.ends_with(b"':") || src.ends_with(b"\":"))
            {
                let value = sym.unescaped();
                if can_be_bare_symbol(value) {
                    let loc = sym.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    let value_str = std::str::from_utf8(value).unwrap_or("symbol");
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Unnecessary symbol conversion; use `{value_str}:` instead."),
                    ));
                }
            }
            return;
        }

        // Check CallNode: .to_sym patterns
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        // Check :symbol.to_sym or "string".to_sym
        if method_name == b"to_sym" {
            if let Some(recv) = call.receiver() {
                if (recv.as_symbol_node().is_some() || recv.as_string_node().is_some())
                    && call.arguments().is_none()
                {
                    let loc = call.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        "Unnecessary symbol conversion detected.".to_string(),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SymbolConversion, "cops/lint/symbol_conversion");
}
