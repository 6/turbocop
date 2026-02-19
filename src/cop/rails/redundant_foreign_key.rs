use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};

pub struct RedundantForeignKey;

impl Cop for RedundantForeignKey {
    fn name(&self) -> &'static str {
        "Rails/RedundantForeignKey"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        if call.receiver().is_some() {
            return;
        }
        if call.name().as_slice() != b"belongs_to" {
            return;
        }
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        // First argument should be a symbol (association name)
        let first_arg = match args.arguments().iter().next() {
            Some(a) => a,
            None => return,
        };
        let assoc_name = match first_arg.as_symbol_node() {
            Some(s) => s.unescaped().to_vec(),
            None => return,
        };
        // Check for foreign_key keyword arg
        let fk_value = match keyword_arg_value(&call, b"foreign_key") {
            Some(v) => v,
            None => return,
        };
        // foreign_key can be a symbol or string
        let fk_name = if let Some(sym) = fk_value.as_symbol_node() {
            sym.unescaped().to_vec()
        } else if let Some(s) = fk_value.as_string_node() {
            s.unescaped().to_vec()
        } else {
            return;
        };
        // Build expected default: "{assoc_name}_id"
        let mut expected = assoc_name;
        expected.extend_from_slice(b"_id");
        if fk_name == expected {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Redundant `foreign_key` -- it matches the default.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantForeignKey, "cops/rails/redundant_foreign_key");
}
