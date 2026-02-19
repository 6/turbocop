use crate::cop::util;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE};

pub struct EagerEvaluationLogMessage;

impl Cop for EagerEvaluationLogMessage {
    fn name(&self) -> &'static str {
        "Rails/EagerEvaluationLogMessage"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE]
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

        if call.name().as_slice() != b"debug" {
            return Vec::new();
        }

        // If already using a block, skip
        if call.block().is_some() {
            return Vec::new();
        }

        // Receiver must be Rails.logger (a 2-method chain)
        let chain = match util::as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        if chain.inner_method != b"logger" {
            return Vec::new();
        }

        // Inner receiver must be `Rails` constant
        let inner_recv = match chain.inner_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let is_rails = if let Some(cr) = inner_recv.as_constant_read_node() {
            cr.name().as_slice() == b"Rails"
        } else if let Some(cp) = inner_recv.as_constant_path_node() {
            // ::Rails
            cp.parent().is_none()
                && cp.name().map_or(false, |n| n.as_slice() == b"Rails")
        } else {
            false
        };

        if !is_rails {
            return Vec::new();
        }

        // First argument must be an interpolated string (dstr)
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // Check if the first argument is an interpolated string
        if arg_list[0].as_interpolated_string_node().is_none() {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Pass a block to `Rails.logger.debug`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EagerEvaluationLogMessage, "cops/rails/eager_evaluation_log_message");
}
