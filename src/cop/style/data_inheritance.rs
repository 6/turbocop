use crate::cop::node_type::{CALL_NODE, CLASS_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DataInheritance;

impl Cop for DataInheritance {
    fn name(&self) -> &'static str {
        "Style/DataInheritance"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            CLASS_NODE,
            CONSTANT_PATH_NODE,
            CONSTANT_READ_NODE,
        ]
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
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return,
        };

        // Must have a superclass
        let superclass = match class_node.superclass() {
            Some(s) => s,
            None => return,
        };

        // Check if superclass is Data.define(...)
        if is_data_define(&superclass) {
            let loc = superclass.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Don't extend an instance initialized by `Data.define`. Use a block to customize the class.".to_string(),
            ));
        }
    }
}

fn is_data_define(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if call.name().as_slice() != b"define" {
        return false;
    }

    match call.receiver() {
        Some(recv) => is_data_const(&recv),
        None => false,
    }
}

fn is_data_const(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(c) = node.as_constant_read_node() {
        return c.name().as_slice() == b"Data";
    }
    if let Some(cp) = node.as_constant_path_node() {
        return cp.parent().is_none() && cp.name().is_some_and(|n| n.as_slice() == b"Data");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DataInheritance, "cops/style/data_inheritance");
}
