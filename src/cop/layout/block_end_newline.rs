use crate::cop::shared::node_type::{BLOCK_NODE, LAMBDA_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-10)
///
/// CI baseline reported FP=0, FN=1.
///
/// Earlier in this department pass, multiline lambdas like `-> { ... }` and
/// `-> do ... end` were fixed by widening the visitor set from `BLOCK_NODE` to
/// `LAMBDA_NODE` while preserving RuboCop's `; end` / `; }` escape.
///
/// Fixed the remaining FN=1: block-local parameter continuations such as
/// `foo { |\n;x| }` were incorrectly accepted because the escape treated any
/// trimmed line starting with `;` as the allowed `; }` form. The accepted fix
/// now only skips when the semicolon is followed solely by whitespace.
///
/// Acceptance gate after this patch (`scripts/check-cop.py --verbose --rerun`):
/// expected=1,446, actual=1,477, CI baseline=1,445, raw excess=31,
/// missing=0, file-drop noise=45. The rerun passes against the CI baseline
/// once that existing parser-crash noise is applied.
///
/// ## Corpus investigation (2026-03-19)
///
/// FP=1 root cause:
/// - RuboCop's semicolon escape checks only the trailing segment from the last
///   block child to the closing delimiter (`node.children.compact.last...join`),
///   not the entire line. In corpus code like:
///   `Module.new {\n  def self.release\n    "1.0"\n  end; }`
///   the relevant trailing segment is just `; }`, so RuboCop accepts it.
///   Nitrocop was checking from the start of the line, seeing `end; }`, and
///   reporting a false positive. Fixed by deriving the trailing segment from
///   the block's last body expression (or block params when there is no body)
///   before applying the `; end` / `; }` escape.
pub struct BlockEndNewline;

impl Cop for BlockEndNewline {
    fn name(&self) -> &'static str {
        "Layout/BlockEndNewline"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, LAMBDA_NODE]
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
        let (opening_loc, closing_loc) = if let Some(block_node) = node.as_block_node() {
            (block_node.opening_loc(), block_node.closing_loc())
        } else if let Some(lambda_node) = node.as_lambda_node() {
            (lambda_node.opening_loc(), lambda_node.closing_loc())
        } else {
            return;
        };

        let (open_line, _) = source.offset_to_line_col(opening_loc.start_offset());
        let (close_line, close_col) = source.offset_to_line_col(closing_loc.start_offset());

        // Single line block — no offense
        if open_line == close_line {
            return;
        }

        // Check if `end` or `}` begins its line (only whitespace before it)
        let bytes = source.as_bytes();
        let mut pos = closing_loc.start_offset();
        while pos > 0 && bytes[pos - 1] != b'\n' {
            pos -= 1;
        }

        // Check if everything from line start to closing is whitespace
        let before_close = &bytes[pos..closing_loc.start_offset()];
        let begins_line = before_close.iter().all(|&b| b == b' ' || b == b'\t');

        if begins_line || has_allowed_semicolon_escape(node, bytes, closing_loc.start_offset()) {
            return;
        }

        diagnostics.push(self.diagnostic(
            source,
            close_line,
            close_col,
            format!(
                "Expression at {}, {} should be on its own line.",
                close_line,
                close_col + 1
            ),
        ));
    }
}

fn has_allowed_semicolon_escape(
    node: &ruby_prism::Node<'_>,
    bytes: &[u8],
    closing_start: usize,
) -> bool {
    let Some(trailing_start) = trailing_segment_start_offset(node) else {
        return false;
    };
    if trailing_start >= closing_start {
        return false;
    }

    let trailing = &bytes[trailing_start..closing_start];
    let Some(first_non_whitespace) = trailing
        .iter()
        .position(|&b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
    else {
        return false;
    };

    trailing[first_non_whitespace..].starts_with(b";")
}

fn trailing_segment_start_offset(node: &ruby_prism::Node<'_>) -> Option<usize> {
    if let Some(block_node) = node.as_block_node() {
        if let Some(body) = block_node.body() {
            return Some(last_expression_end_offset(&body));
        }
        if let Some(params) = block_node.parameters() {
            return Some(params.location().end_offset());
        }
    } else if let Some(lambda_node) = node.as_lambda_node() {
        if let Some(body) = lambda_node.body() {
            return Some(last_expression_end_offset(&body));
        }
        if let Some(params) = lambda_node.parameters() {
            return Some(params.location().end_offset());
        }
    }

    None
}

fn last_expression_end_offset(node: &ruby_prism::Node<'_>) -> usize {
    if let Some(stmts) = node.as_statements_node() {
        if let Some(last) = stmts.body().last() {
            return last_expression_end_offset(&last);
        }
    }

    node.location().end_offset()
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(BlockEndNewline, "cops/layout/block_end_newline");
}
