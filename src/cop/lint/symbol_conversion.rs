use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};

pub struct SymbolConversion;

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
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let _style = config.get_str("EnforcedStyle", "strict");

        let method_name = call.name().as_slice();

        // Check :symbol.to_sym
        if method_name == b"to_sym" {
            if let Some(recv) = call.receiver() {
                if recv.as_symbol_node().is_some() && call.arguments().is_none() {
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

        // Check "string".to_sym (converting string literal to symbol)
        if method_name == b"to_sym" {
            if let Some(recv) = call.receiver() {
                if recv.as_string_node().is_some() && call.arguments().is_none() {
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
