use crate::cop::shared::node_type::{CALL_NODE, FORWARDING_SUPER_NODE, SUPER_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/EmptyBlock — checks for blocks without a body.
///
/// ## Corpus investigation (2026-03-15)
///
/// Previous fixes closed the large `Proc.new {}` false-positive cluster by
/// matching RuboCop's `lambda_or_proc?` handling for `Proc.new` and
/// `::Proc.new`. Also added `SuperNode` handling for `super(...) {}`.
///
/// ## Corpus investigation (2026-03-21)
///
/// Fixed 3 FN (DmitryTsepelev/graphql-ruby-persisted_queries, coyote-team/coyote,
/// thoughtbot/props_template) for `super {}` without parentheses. In Ruby's Prism
/// parser, `super {}` (without explicit parentheses) is parsed as `ForwardingSuperNode`,
/// not `SuperNode`. `SuperNode` is used only for `super(...)` with explicit parens.
/// Fix: added `ForwardingSuperNode` to `interested_node_types()` and added handling
/// branch in `check_node()` that mirrors the existing `SuperNode` logic.
///
/// ## Corpus investigation (2026-03-16)
///
/// Fixed 15 FP / 13 FN from chained block location mismatch. For code like
/// `create_table(...) do...end.define_model do end`, nitrocop was reporting at
/// the `define_model` message location instead of at the block expression start.
/// RuboCop's `on_block` receives the outer BlockNode whose expression starts at
/// the full receiver chain (`Context.create_table`). Fix: always use
/// `call_node.location().start_offset()` (which includes the receiver) instead
/// of `message_loc()` for chained calls. All 28 mismatches (concentrated in
/// gregnavis/active_record_doctor) were paired FP+FN from this single location
/// bug.
///
/// ## Corpus investigation (2026-03-16, follow-up)
///
/// Fixed 1 remaining FP in gregnavis/active_record_doctor. The pattern:
/// `create_table(...) do |t| # comment ... end.define_model do end` — the
/// `define_model` block is empty, but comments exist in the earlier `create_table`
/// block. RuboCop's `allow_comment?` checks the full block expression source range
/// (which includes the receiver chain and its block), so comments inside the
/// `create_table` block suppress the offense. Fix: expanded the AllowComments
/// comment search range from `block_node.location()` (just `do...end`) to the
/// full expression range (call_node start through block_node end).
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
    rest.split(',').any(|part| {
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
        &[CALL_NODE, SUPER_NODE, FORWARDING_SUPER_NODE]
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
        let (call_node, super_node, forwarding_super_node, block_node) =
            if let Some(call_node) = node.as_call_node() {
                let block_node = match call_node.block().and_then(|b| b.as_block_node()) {
                    Some(bn) => bn,
                    None => return, // BlockArgumentNode — not a literal block
                };
                (Some(call_node), None, None, block_node)
            } else if let Some(super_node) = node.as_super_node() {
                let block_node = match super_node.block().and_then(|b| b.as_block_node()) {
                    Some(bn) => bn,
                    None => return,
                };
                (None, Some(super_node), None, block_node)
            } else if let Some(forwarding_super_node) = node.as_forwarding_super_node() {
                let block_node = match forwarding_super_node.block() {
                    Some(bn) => bn,
                    None => return,
                };
                (None, None, Some(forwarding_super_node), block_node)
            } else {
                return;
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
        // RuboCop's lambda_or_proc? covers: lambda {}, proc {}, Proc.new {}, ::Proc.new {}
        let allow_empty_lambdas = config.get_bool("AllowEmptyLambdas", true);
        if allow_empty_lambdas {
            if let Some(call_node) = call_node.as_ref() {
                let name = call_node.name().as_slice();
                if (name == b"lambda" || name == b"proc") && call_node.receiver().is_none() {
                    return;
                }
                // Proc.new {} and ::Proc.new {}
                if name == b"new" {
                    if let Some(receiver) = call_node.receiver() {
                        let is_proc_const = receiver
                            .as_constant_read_node()
                            .is_some_and(|c| c.name().as_slice() == b"Proc")
                            || receiver.as_constant_path_node().is_some_and(|cp| {
                                cp.parent().is_none()
                                    && cp.name().is_some_and(|n| n.as_slice() == b"Proc")
                            });
                        if is_proc_const {
                            return;
                        }
                    }
                }
            }
        }

        // AllowComments: when true, blocks with comments on or inside them are not offenses.
        // RuboCop checks for any comment within the block's source range OR on the same line,
        // UNLESS the comment is a rubocop:disable directive for this specific cop.
        let allow_comments = config.get_bool("AllowComments", true);
        if allow_comments {
            // RuboCop checks the full block expression range (including receiver)
            // for comments. For chained calls like `create_table(...) do...end.define_model do end`,
            // the outer BlockNode in RuboCop spans the entire expression. We replicate this by
            // using the call_node/super_node start through block_node end.
            let start_offset = if let Some(cn) = call_node.as_ref() {
                cn.location().start_offset()
            } else if let Some(ref sn) = super_node {
                sn.location().start_offset()
            } else if let Some(ref fsn) = forwarding_super_node {
                fsn.location().start_offset()
            } else {
                block_node.location().start_offset()
            };
            let end_offset = block_node.location().end_offset();
            let (start_line, _) = source.offset_to_line_col(start_offset);
            let (end_line, _) = source.offset_to_line_col(end_offset.saturating_sub(1));

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

        let diagnostic_offset = if let Some(call_node) = call_node.as_ref() {
            call_node.location().start_offset()
        } else if let Some(super_node) = super_node {
            super_node.location().start_offset()
        } else if let Some(forwarding_super_node) = forwarding_super_node {
            forwarding_super_node.location().start_offset()
        } else {
            block_node.location().start_offset()
        };

        let (line, column) = source.offset_to_line_col(diagnostic_offset);
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
