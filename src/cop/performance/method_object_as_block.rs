use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE};

pub struct MethodObjectAsBlock;

impl Cop for MethodObjectAsBlock {
    fn name(&self) -> &'static str {
        "Performance/MethodObjectAsBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Detect BlockArgumentNode whose expression is a call to `method`
        let block_arg = match node.as_block_argument_node() {
            Some(b) => b,
            None => return,
        };

        let expr = match block_arg.expression() {
            Some(e) => e,
            None => return,
        };

        let call = match expr.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"method" {
            return;
        }

        let loc = block_arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, "Use a block instead of `&method(...)` for better performance.".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MethodObjectAsBlock, "cops/performance/method_object_as_block");
}
