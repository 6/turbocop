use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct BelongsTo;

impl Cop for BelongsTo {
    fn name(&self) -> &'static str {
        "Rails/BelongsTo"
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

        if call.receiver().is_some() || call.name().as_slice() != b"belongs_to" {
            return Vec::new();
        }

        // Check for `required:` keyword argument
        let required_value = match keyword_arg_value(&call, b"required") {
            Some(v) => v,
            None => return Vec::new(),
        };

        let message = if required_value.as_true_node().is_some() {
            "You specified `required: true`, in Rails > 5.0 the required option is deprecated and you want to use `optional: false`."
        } else if required_value.as_false_node().is_some() {
            "You specified `required: false`, in Rails > 5.0 the required option is deprecated and you want to use `optional: true`."
        } else {
            return Vec::new();
        };

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, message.to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BelongsTo, "cops/rails/belongs_to");
}
