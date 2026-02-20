use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{ARRAY_NODE, CALL_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE, HASH_NODE, INTERPOLATED_STRING_NODE, KEYWORD_HASH_NODE, STRING_NODE};

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

    fn is_string_literal(node: &ruby_prism::Node<'_>) -> bool {
        node.as_string_node().is_some() || node.as_interpolated_string_node().is_some()
    }

    fn is_frozen_call(node: &ruby_prism::Node<'_>) -> bool {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"freeze" {
                return true;
            }
        }
        false
    }

    /// Check if the source file has `# frozen_string_literal: true` in the
    /// first few lines (before any code). This magic comment makes all string
    /// literals frozen, so `.freeze` on string constants is unnecessary.
    fn has_frozen_string_literal_true(source: &SourceFile) -> bool {
        let lines = source.lines();
        // Check up to the first 3 lines (shebang, encoding, magic comment)
        for (i, line) in lines.enumerate() {
            if i >= 3 {
                break;
            }
            let s = match std::str::from_utf8(line) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let s = s.trim();
            if s.is_empty() {
                continue;
            }
            if let Some(rest) = s.strip_prefix('#') {
                let rest = rest.trim_start();
                if let Some(value) = rest.strip_prefix("frozen_string_literal:") {
                    return value.trim() == "true";
                }
            }
        }
        false
    }

    fn check_value(
        &self,
        source: &SourceFile,
        value: &ruby_prism::Node<'_>,
        name_offset: usize,
        frozen_strings: bool,
    ) -> Vec<Diagnostic> {
        if !Self::is_mutable_literal(value) || Self::is_frozen_call(value) {
            return Vec::new();
        }

        // When frozen_string_literal: true is set, plain string constants
        // are already frozen — don't flag them.
        if frozen_strings && Self::is_string_literal(value) {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(name_offset);
        vec![self.diagnostic(
            source,
            line,
            column,
            "Freeze mutable objects assigned to constants.".to_string(),
        )]
    }
}

impl Cop for MutableConstant {
    fn name(&self) -> &'static str {
        "Style/MutableConstant"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[ARRAY_NODE, CALL_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE, HASH_NODE, INTERPOLATED_STRING_NODE, KEYWORD_HASH_NODE, STRING_NODE]
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
        let _enforced_style = config.get_str("EnforcedStyle", "literals");

        let frozen_strings = Self::has_frozen_string_literal_true(source);

        // Check ConstantWriteNode (CONST = value)
        if let Some(cw) = node.as_constant_write_node() {
            let value = cw.value();
            diagnostics.extend(self.check_value(
                source,
                &value,
                cw.name_loc().start_offset(),
                frozen_strings,
            ));
            return;
        }

        // Check ConstantPathWriteNode (Module::CONST = value)
        if let Some(cpw) = node.as_constant_path_write_node() {
            let value = cpw.value();
            let target = cpw.target();
            diagnostics.extend(self.check_value(
                source,
                &value,
                target.location().start_offset(),
                frozen_strings,
            ));
            return;
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MutableConstant, "cops/style/mutable_constant");
}
