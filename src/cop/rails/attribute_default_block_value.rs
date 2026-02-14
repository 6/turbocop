use crate::cop::util::{is_dsl_call, keyword_arg_value};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AttributeDefaultBlockValue;

impl Cop for AttributeDefaultBlockValue {
    fn name(&self) -> &'static str {
        "Rails/AttributeDefaultBlockValue"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if !is_dsl_call(&call, b"attribute") {
            return Vec::new();
        }

        // Check if :default keyword arg exists
        let default_value = match keyword_arg_value(&call, b"default") {
            Some(v) => v,
            None => return Vec::new(),
        };

        // Flag mutable default values that should use a block:
        // Arrays, Hashes, and String literals are mutable
        let is_mutable = default_value.as_array_node().is_some()
            || default_value.as_hash_node().is_some()
            || default_value.as_string_node().is_some();

        if is_mutable {
            let loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Pass a block to `default:` to avoid sharing mutable objects.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        AttributeDefaultBlockValue,
        "cops/rails/attribute_default_block_value"
    );
}
