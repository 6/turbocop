use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct LambdaWithoutLiteralBlock;

impl Cop for LambdaWithoutLiteralBlock {
    fn name(&self) -> &'static str {
        "Lint/LambdaWithoutLiteralBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for `lambda(&something)` — lambda called with a block argument
        // instead of a literal block.
        //
        // In Prism, `lambda(&pr)` is a CallNode where:
        // - method name is "lambda"
        // - no receiver
        // - has a block_argument (BlockArgumentNode) instead of a block (BlockNode)
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"lambda" {
            return Vec::new();
        }

        // Must have no receiver (bare `lambda`)
        if call.receiver().is_some() {
            return Vec::new();
        }

        // Check if it has a block — if it has a literal block, that's fine
        if call.block().is_some() {
            return Vec::new();
        }

        // Check arguments for a block argument (&something)
        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        let has_block_arg = args.iter().any(|a| a.as_block_argument_node().is_some());

        if !has_block_arg {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "lambda without a literal block is deprecated; use the proc without lambda instead."
                .to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LambdaWithoutLiteralBlock, "cops/lint/lambda_without_literal_block");
}
