use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MutableConstant;

impl MutableConstant {
    fn is_mutable_literal(node: &ruby_prism::Node<'_>) -> bool {
        // Arrays, hashes (including keyword hashes), and strings (when not frozen) are mutable
        node.as_array_node().is_some()
            || node.as_hash_node().is_some()
            || node.as_keyword_hash_node().is_some()
            || node.as_string_node().is_some()
            || node.as_interpolated_string_node().is_some()
    }

    fn is_frozen_call(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"freeze" {
                return true;
            }
        }
        false
    }
}

impl Cop for MutableConstant {
    fn name(&self) -> &'static str {
        "Style/MutableConstant"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _enforced_style = config.get_str("EnforcedStyle", "literals");

        // Check ConstantWriteNode (CONST = value)
        if let Some(cw) = node.as_constant_write_node() {
            let value = cw.value();
            if Self::is_mutable_literal(&value) && !Self::is_frozen_call(&value) {
                let loc = cw.name_loc();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Freeze mutable objects assigned to constants.".to_string(),
                )];
            }
        }

        // Check ConstantPathWriteNode (Module::CONST = value)
        if let Some(cpw) = node.as_constant_path_write_node() {
            let value = cpw.value();
            if Self::is_mutable_literal(&value) && !Self::is_frozen_call(&value) {
                let target = cpw.target();
                let loc = target.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Freeze mutable objects assigned to constants.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MutableConstant, "cops/style/mutable_constant");
}
