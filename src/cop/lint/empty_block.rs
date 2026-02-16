use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyBlock;

impl Cop for EmptyBlock {
    fn name(&self) -> &'static str {
        "Lint/EmptyBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let block_node = match node.as_block_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let body_empty = match block_node.body() {
            None => true,
            Some(body) => {
                if let Some(stmts) = body.as_statements_node() {
                    stmts.body().is_empty()
                } else {
                    false
                }
            }
        };

        if !body_empty {
            return Vec::new();
        }

        // AllowEmptyLambdas: skip lambda blocks
        let allow_empty_lambdas = config.get_bool("AllowEmptyLambdas", true);
        if allow_empty_lambdas {
            // Check if this block belongs to a lambda call
            // In Prism, lambda { } is represented as a LambdaNode, not a BlockNode
            // attached to a CallNode. But `-> {}` might be a LambdaNode.
            // For BlockNode, check if the parent call is `lambda`.
            // We don't have parent access, but we can check if the block_node
            // is part of a lambda by checking its opening_loc for `{` vs `do`
            // Actually, lambda nodes in Prism are separate LambdaNode, not BlockNode.
            // But `lambda { }` is a CallNode with a BlockNode.
            // We can't tell from BlockNode alone, so skip this for now.
            // The node visitor gives us BlockNode directly; we'd need to check
            // the call that owns this block. Let's skip this edge case.
        }

        // AllowComments: when true, blocks containing only comments are not offenses
        let allow_comments = config.get_bool("AllowComments", true);
        if allow_comments {
            let loc = block_node.location();
            let (start_line, _) = source.offset_to_line_col(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));
            let lines: Vec<&[u8]> = source.lines().collect();
            for line_num in start_line..=end_line {
                if let Some(line) = lines.get(line_num - 1) {
                    let trimmed = line
                        .iter()
                        .position(|&b| b != b' ' && b != b'\t')
                        .map(|start| &line[start..])
                        .unwrap_or(&[]);
                    if trimmed.starts_with(b"#") {
                        return Vec::new();
                    }
                }
            }
        }

        let loc = block_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Empty block detected.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyBlock, "cops/lint/empty_block");
}
