use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, BLOCK_NODE, CALL_NODE, SYMBOL_NODE};

pub struct LambdaWithoutLiteralBlock;

impl Cop for LambdaWithoutLiteralBlock {
    fn name(&self) -> &'static str {
        "Lint/LambdaWithoutLiteralBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, BLOCK_NODE, CALL_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for `lambda(&something)` -- lambda called with a block argument
        // instead of a literal block.
        //
        // In Prism, `lambda(&pr)` parses as a CallNode where:
        // - method name is "lambda"
        // - no receiver
        // - block() returns a BlockArgumentNode (not a BlockNode)
        //
        // `lambda { ... }` parses with block() returning a BlockNode.
        // `lambda(&:do_something)` should NOT be flagged (symbol proc).
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

        // Check if block() is a BlockArgumentNode (not a literal block)
        let block = match call.block() {
            Some(b) => b,
            None => {
                // No block at all -- check arguments for &something
                return self.check_arguments(call, source);
            }
        };

        // If it's a literal block (BlockNode), that's fine
        if block.as_block_node().is_some() {
            return Vec::new();
        }

        // If it's a BlockArgumentNode, check what's inside
        if let Some(block_arg) = block.as_block_argument_node() {
            // Skip symbol procs like `lambda(&:do_something)`
            if let Some(expr) = block_arg.expression() {
                if expr.as_symbol_node().is_some() {
                    return Vec::new();
                }
            }

            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "lambda without a literal block is deprecated; use the proc without lambda instead."
                    .to_string(),
            )];
        }

        Vec::new()
    }
}

impl LambdaWithoutLiteralBlock {
    fn check_arguments(
        &self,
        call: ruby_prism::CallNode<'_>,
        source: &SourceFile,
    ) -> Vec<Diagnostic> {
        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        let has_block_arg = args.iter().any(|a| {
            if let Some(block_arg) = a.as_block_argument_node() {
                // Skip symbol procs
                if let Some(expr) = block_arg.expression() {
                    return !expr.as_symbol_node().is_some();
                }
                return true;
            }
            false
        });

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
