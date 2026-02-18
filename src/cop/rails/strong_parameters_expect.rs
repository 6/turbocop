use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct StrongParametersExpect;

/// Check if a node is a `params` receiver (local variable or method call).
fn is_params_receiver(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        return call.name().as_slice() == b"params" && call.receiver().is_none();
    }
    if let Some(lvar) = node.as_local_variable_read_node() {
        return lvar.name().as_slice() == b"params";
    }
    false
}

impl Cop for StrongParametersExpect {
    fn name(&self) -> &'static str {
        "Rails/StrongParametersExpect"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["**/app/controllers/**/*.rb"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Pattern 1: params.require(:x).permit(:a, :b)
        // Pattern 2: params.permit(x: [:a, :b]).require(:x)
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let is_require_permit = chain.inner_method == b"require" && chain.outer_method == b"permit";
        let is_permit_require = chain.inner_method == b"permit" && chain.outer_method == b"require";

        if !is_require_permit && !is_permit_require {
            return Vec::new();
        }

        // Check if the innermost receiver is `params`
        let inner_receiver = match chain.inner_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if !is_params_receiver(&inner_receiver) {
            return Vec::new();
        }

        // For require.permit, permit must have arguments
        if is_require_permit {
            let outer_call = node.as_call_node().unwrap();
            if outer_call.arguments().is_none() {
                return Vec::new();
            }
        }

        let msg_loc = chain.inner_call.message_loc().unwrap_or(chain.inner_call.location());

        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());

        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `expect(...)` instead.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StrongParametersExpect, "cops/rails/strong_parameters_expect");
}
