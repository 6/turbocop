use crate::cop::node_type::{ARRAY_NODE, SPLAT_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArrayCoercion;

impl Cop for ArrayCoercion {
    fn name(&self) -> &'static str {
        "Style/ArrayCoercion"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, SPLAT_NODE]
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
        // Pattern 1: [*var] - splat into array with single element
        if let Some(array_node) = node.as_array_node() {
            // Skip implicit arrays (e.g., RHS of multi-write `a, b = *x`)
            if array_node.opening_loc().is_none() {
                return;
            }
            let elements: Vec<_> = array_node.elements().iter().collect();
            if elements.len() == 1 && elements[0].as_splat_node().is_some() {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `Array(variable)` instead of `[*variable]`.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArrayCoercion, "cops/style/array_coercion");
}
