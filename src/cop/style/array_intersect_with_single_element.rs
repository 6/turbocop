use crate::cop::node_type::{ARRAY_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ArrayIntersectWithSingleElement;

impl Cop for ArrayIntersectWithSingleElement {
    fn name(&self) -> &'static str {
        "Style/ArrayIntersectWithSingleElement"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = std::str::from_utf8(call.name().as_slice()).unwrap_or("");
        if method_name != "intersect?" {
            return;
        }

        if call.receiver().is_none() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // Check if the argument is a single-element array literal
        if let Some(array_node) = arg_list[0].as_array_node() {
            let elements: Vec<_> = array_node.elements().iter().collect();
            if elements.len() == 1 {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Use `include?(element)` instead of `intersect?([element])`.".to_string(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ArrayIntersectWithSingleElement,
        "cops/style/array_intersect_with_single_element"
    );
}
