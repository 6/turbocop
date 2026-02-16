use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EachForSimpleLoop;

impl Cop for EachForSimpleLoop {
    fn name(&self) -> &'static str {
        "Style/EachForSimpleLoop"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Look for CallNode with .each and a block
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call_node.name().as_slice() != b"each" {
            return Vec::new();
        }

        // Must have a block
        let block = match call_node.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Block must have no parameters (empty args) or no params node at all
        if let Some(params) = block_node.parameters() {
            if let Some(bp) = params.as_block_parameters_node() {
                if let Some(inner_params) = bp.parameters() {
                    // Check all param lists are empty
                    let has_params = !inner_params.requireds().is_empty()
                        || !inner_params.optionals().is_empty()
                        || inner_params.rest().is_some()
                        || !inner_params.posts().is_empty()
                        || !inner_params.keywords().is_empty()
                        || inner_params.keyword_rest().is_some()
                        || inner_params.block().is_some();
                    if has_params {
                        return Vec::new();
                    }
                }
            } else {
                // Some other parameter type - skip
                return Vec::new();
            }
        }

        // Receiver must be a parenthesized range: (0..n) or (1..n)
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Unwrap parentheses
        let parens = match receiver.as_parentheses_node() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let parens_body = match parens.body() {
            Some(body) => body,
            None => return Vec::new(),
        };

        // The body may be a RangeNode directly or wrapped in a StatementsNode
        let range_node = if let Some(r) = parens_body.as_range_node() {
            r
        } else if let Some(stmts) = parens_body.as_statements_node() {
            let body: Vec<_> = stmts.body().iter().collect();
            if body.len() != 1 {
                return Vec::new();
            }
            match body[0].as_range_node() {
                Some(r) => r,
                None => return Vec::new(),
            }
        } else {
            return Vec::new();
        };

        // Left side must be an integer literal
        let left = match range_node.left() {
            Some(l) => l,
            None => return Vec::new(),
        };

        if left.as_integer_node().is_none() {
            return Vec::new();
        }

        // Right side must be an integer literal
        let right = match range_node.right() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if right.as_integer_node().is_none() {
            return Vec::new();
        }

        // Check if left is 0 (for inclusive range) or args are empty
        // We flag all cases with integer ranges and empty block params
        let (line, column) = source.offset_to_line_col(receiver.location().start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `Integer#times` for a simple loop which iterates a fixed number of times."
                .to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EachForSimpleLoop, "cops/style/each_for_simple_loop");
}
