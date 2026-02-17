use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct StructInheritance;

impl Cop for StructInheritance {
    fn name(&self) -> &'static str {
        "Style/StructInheritance"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let class_node = match node.as_class_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have a superclass
        let superclass = match class_node.superclass() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Check if superclass is Struct.new(...) or ::Struct.new(...)
        // It could be a direct CallNode or a block wrapping a CallNode
        if is_struct_new(&superclass) || is_struct_new_block(&superclass) {
            let loc = superclass.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Don't extend an instance initialized by `Struct.new`. Use a block to customize the struct.".to_string(),
            )];
        }

        Vec::new()
    }
}

fn is_struct_new(node: &ruby_prism::Node<'_>) -> bool {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return false,
    };

    if call.name().as_slice() != b"new" {
        return false;
    }

    match call.receiver() {
        Some(recv) => is_struct_const(&recv),
        None => false,
    }
}

fn is_struct_new_block(node: &ruby_prism::Node<'_>) -> bool {
    // block { Struct.new(...) do ... end }
    // Prism represents this as a CallNode with a block
    // Actually let's check if this is a block whose call is Struct.new
    if let Some(call) = node.as_call_node() {
        if let Some(block) = call.block() {
            // The call itself is Struct.new, and there's a block
            if call.name().as_slice() == b"new" {
                if let Some(recv) = call.receiver() {
                    if is_struct_const(&recv) {
                        let _ = block;
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn is_struct_const(node: &ruby_prism::Node<'_>) -> bool {
    // Check for `Struct` (ConstantReadNode) or `::Struct` (ConstantPathNode)
    if let Some(c) = node.as_constant_read_node() {
        return c.name().as_slice() == b"Struct";
    }
    if let Some(cp) = node.as_constant_path_node() {
        return cp.parent().is_none()
            && cp.name().is_some_and(|n| n.as_slice() == b"Struct");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StructInheritance, "cops/style/struct_inheritance");
}
