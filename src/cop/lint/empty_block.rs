use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct EmptyBlock;

/// Check if a comment is a rubocop:disable directive for a specific cop.
fn is_disable_comment_for_cop(comment_bytes: &[u8], cop_name: &[u8]) -> bool {
    // Match patterns like: # rubocop:disable Lint/EmptyBlock
    // or: # rubocop:todo Lint/EmptyBlock
    // Whitespace between tokens is flexible.
    let s = match std::str::from_utf8(comment_bytes) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let cop = match std::str::from_utf8(cop_name) {
        Ok(s) => s,
        Err(_) => return false,
    };
    // Strip leading # and whitespace
    let s = s.trim_start_matches('#').trim();
    // Check for rubocop:disable or rubocop:todo prefix
    let rest = if let Some(r) = s.strip_prefix("rubocop:disable") {
        r
    } else if let Some(r) = s.strip_prefix("rubocop:todo") {
        r
    } else {
        return false;
    };
    let rest = rest.trim();
    // Check if the cop name or "all" is in the comma-separated list
    rest.split(',')
        .any(|part| {
            let part = part.trim();
            part == cop || part == "all"
        })
}

impl Cop for EmptyBlock {
    fn name(&self) -> &'static str {
        "Lint/EmptyBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let block_node = match call_node.block() {
            Some(b) => match b.as_block_node() {
                Some(bn) => bn,
                None => return, // BlockArgumentNode — not a literal block
            },
            None => return,
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
            return;
        }

        // AllowEmptyLambdas: skip lambda/proc blocks
        let allow_empty_lambdas = config.get_bool("AllowEmptyLambdas", true);
        if allow_empty_lambdas {
            let name = call_node.name().as_slice();
            if (name == b"lambda" || name == b"proc") && call_node.receiver().is_none() {
                return;
            }
        }

        // AllowComments: when true, blocks with comments on or inside them are not offenses.
        // RuboCop checks for any comment within the block's source range OR on the same line,
        // UNLESS the comment is a rubocop:disable directive for this specific cop.
        let allow_comments = config.get_bool("AllowComments", true);
        if allow_comments {
            let loc = block_node.location();
            let (start_line, _) = source.offset_to_line_col(loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(loc.end_offset().saturating_sub(1));

            for comment in parse_result.comments() {
                let comment_offset = comment.location().start_offset();
                let (comment_line, _) = source.offset_to_line_col(comment_offset);
                if comment_line >= start_line && comment_line <= end_line {
                    // Found a comment on the block's lines.
                    // Skip if the comment is a rubocop:disable for this cop
                    // (the disable mechanism handles that separately).
                    let comment_text = comment.location().as_slice();
                    if !is_disable_comment_for_cop(comment_text, b"Lint/EmptyBlock") {
                        return;
                    }
                }
            }
        }

        // Use the call node's location for the diagnostic (matches RuboCop's block node
        // which in Parser AST spans the entire expression including the receiver).
        let loc = call_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Empty block detected.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyBlock, "cops/lint/empty_block");
}
