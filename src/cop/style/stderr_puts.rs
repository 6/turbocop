use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, GLOBAL_VARIABLE_READ_NODE};

pub struct StderrPuts;

impl Cop for StderrPuts {
    fn name(&self) -> &'static str {
        "Style/StderrPuts"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, GLOBAL_VARIABLE_READ_NODE]
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

        // Must be `puts` method
        if call.name().as_slice() != b"puts" {
            return;
        }

        // Must have at least one argument
        if call.arguments().is_none() {
            return;
        }

        // Receiver must be $stderr or STDERR or ::STDERR
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_stderr_gvar = receiver
            .as_global_variable_read_node()
            .is_some_and(|g| g.name().as_slice() == b"$stderr");

        let is_stderr_const = receiver
            .as_constant_read_node()
            .is_some_and(|c| c.name().as_slice() == b"STDERR");

        let is_stderr_const_path = receiver.as_constant_path_node().is_some_and(|cp| {
            // ::STDERR — parent is None (cbase), name is STDERR
            cp.parent().is_none()
                && cp.name().is_some_and(|n| n.as_slice() == b"STDERR")
        });

        if !is_stderr_gvar && !is_stderr_const && !is_stderr_const_path {
            return;
        }

        let receiver_src = std::str::from_utf8(receiver.location().as_slice()).unwrap_or("");
        let msg = format!(
            "Use `warn` instead of `{}.puts` to allow such output to be disabled.",
            receiver_src
        );

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StderrPuts, "cops/style/stderr_puts");
}
