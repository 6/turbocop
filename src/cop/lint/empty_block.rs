use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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

/// Check if the block starting at `block_start` offset is attached to a `lambda` or `proc` call.
/// Scans backwards from the block's location to find the preceding identifier.
fn is_lambda_or_proc_block(bytes: &[u8], block_start: usize) -> bool {
    // The BlockNode's location starts at `do` or `{`. We need to find what
    // comes before it. The pattern is: `lambda do` or `lambda {` or `proc do`, etc.
    // There may be block parameters between the call and the block keyword:
    // `lambda do |arg|` — but the BlockNode location starts at `do`, not `|`.
    // Actually, for `lambda do |_processed_source| end`, the block location
    // covers `do |_processed_source| end`. Before `do` we should find `lambda `.
    let mut pos = block_start;
    // Skip backwards over whitespace
    while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
        pos -= 1;
    }
    // Now we should be at the end of the preceding identifier (e.g., 'a' of 'lambda')
    let end = pos;
    // Scan backwards over word characters
    while pos > 0 && (bytes[pos - 1].is_ascii_alphanumeric() || bytes[pos - 1] == b'_') {
        pos -= 1;
    }
    let word = &bytes[pos..end];
    word == b"lambda" || word == b"proc"
}

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
        parse_result: &ruby_prism::ParseResult<'_>,
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

        // AllowEmptyLambdas: skip lambda/proc blocks
        let allow_empty_lambdas = config.get_bool("AllowEmptyLambdas", true);
        if allow_empty_lambdas {
            // Check if this block is attached to a `lambda` or `proc` call.
            // Since BlockNode doesn't have a parent reference, scan backwards
            // from the block start to find the preceding call name.
            let block_start = block_node.location().start_offset();
            let bytes = source.as_bytes();
            if is_lambda_or_proc_block(bytes, block_start) {
                return Vec::new();
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
