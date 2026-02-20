use crate::cop::util::keyword_arg_value;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, FALSE_NODE, TRUE_NODE};

pub struct BelongsTo;

impl Cop for BelongsTo {
    fn name(&self) -> &'static str {
        "Rails/BelongsTo"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FALSE_NODE, TRUE_NODE]
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

        if call.receiver().is_some() || call.name().as_slice() != b"belongs_to" {
            return;
        }

        // Check for `required:` keyword argument
        let required_value = match keyword_arg_value(&call, b"required") {
            Some(v) => v,
            None => return,
        };

        let message = if required_value.as_true_node().is_some() {
            "You specified `required: true`, in Rails > 5.0 the required option is deprecated and you want to use `optional: false`."
        } else if required_value.as_false_node().is_some() {
            "You specified `required: false`, in Rails > 5.0 the required option is deprecated and you want to use `optional: true`."
        } else {
            return;
        };

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, message.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BelongsTo, "cops/rails/belongs_to");
}
