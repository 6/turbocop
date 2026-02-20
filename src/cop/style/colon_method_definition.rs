use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::DEF_NODE;

pub struct ColonMethodDefinition;

impl Cop for ColonMethodDefinition {
    fn name(&self) -> &'static str {
        "Style/ColonMethodDefinition"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Must be a singleton method (has receiver: def self::bar)
        if def_node.receiver().is_none() {
            return;
        }

        // Check the operator between receiver and method name
        let operator_loc = match def_node.operator_loc() {
            Some(loc) => loc,
            None => return,
        };

        if operator_loc.as_slice() != b"::" {
            return;
        }

        let (line, column) = source.offset_to_line_col(operator_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not use `::` for defining class methods.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ColonMethodDefinition, "cops/style/colon_method_definition");
}
