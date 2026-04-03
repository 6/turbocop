use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-07)
///
/// Corpus oracle reported FP=102, FN=317.
///
/// FP root causes:
/// 1) Non-symbol `receive` args (e.g., `receive('action_name')`) were matched.
/// 2) The matcher accepted chains RuboCop excludes, including:
///    - heredoc returns (`and_return(<<~SQL)`)
///    - splat returns (`and_return(*values)`)
///    - multi-arg returns (`and_return(1, 2)`)
///    - calls with additional chained methods after `and_return` (e.g., `.ordered`)
///
/// Fix: mirror RuboCop's node pattern shape exactly:
/// `allow(...).to receive(:symbol).and_return(single_non_heredoc_non_splat_arg)`.
///
/// ## Corpus investigation (2026-03-10)
///
/// FN root causes (297 remaining):
/// 1) Only `BlockNode` bodies were checked. RuboCop uses `on_begin` which
///    fires for method bodies, class bodies, begin blocks, etc. — all
///    represented as `StatementsNode` in Prism. Switched to `check_source`
///    with a visitor that processes all `StatementsNode` contexts.
/// 2) Duplicate filtering logic (`has_dups`) was too aggressive: it skipped
///    the *entire group* when any receive arg was duplicated. RuboCop's
///    `uniq_items` only excludes items whose receive arg appears on a
///    *different* line, keeping items with unique args in the group.
///    Fixed to match RuboCop's per-item filtering.
///
/// ## Corpus investigation (2026-03-25)
///
/// FP=3 from two repos: skylightio/skylight-ruby (2 FPs) and jruby/jruby-rack (1 FP).
///
/// FP=2 (skylight): Stubs inside explicit `begin...end` blocks were incorrectly
/// flagged. In the parser gem AST, explicit `begin...end` creates `kwbegin` nodes,
/// and RuboCop's `on_begin` callback does NOT fire on `kwbegin` — only on implicit
/// `begin` nodes (block bodies, method bodies, etc.). Fixed by overriding
/// `visit_begin_node` to skip `check_statements` for BeginNode bodies.
///
/// FP=1 (jruby): Two stubs on the same line separated by `;` were flagged.
/// RuboCop's `repeated_lines - [item.first_line]` yields an empty list when all
/// items share the same line, causing the offense to be skipped. Fixed by checking
/// that unique items span at least 2 distinct lines before reporting.
///
/// ## Corpus investigation (2026-03-29)
///
/// FN=15 clustered in `begin ... ensure/end` and `begin ... rescue/end`
/// bodies. Prism uses `BeginNode` for both bare explicit `begin...end` and
/// handled `begin` blocks, but RuboCop only skips the bare `kwbegin` form.
/// Fixed by skipping only pure explicit `begin...end` bodies with no
/// rescue/else/ensure clauses.
pub struct ReceiveMessages;

struct StubInfo {
    receiver_text: String,
    receive_msg: String,
    offset: usize,
    line: usize,
}

impl Cop for ReceiveMessages {
    fn name(&self) -> &'static str {
        "RSpec/ReceiveMessages"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = ReceiveMessagesVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            pending_begin_body: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ReceiveMessagesVisitor<'a> {
    cop: &'a ReceiveMessages,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Incremented when entering a pure explicit `begin...end` so the next
    /// `visit_statements_node` call skips `check_statements` for that body only.
    pending_begin_body: usize,
}

impl<'a> ReceiveMessagesVisitor<'a> {
    fn check_statements(&mut self, stmts: &ruby_prism::StatementsNode<'_>) {
        let mut stubs: Vec<StubInfo> = Vec::new();

        for stmt in stmts.body().iter() {
            if let Some(info) = extract_allow_receive_info(self.source, &stmt) {
                stubs.push(info);
            }
        }

        // Group by receiver text
        let mut processed = vec![false; stubs.len()];

        for i in 0..stubs.len() {
            if processed[i] {
                continue;
            }

            let mut group = vec![i];
            for j in (i + 1)..stubs.len() {
                if processed[j] {
                    continue;
                }
                if stubs[i].receiver_text == stubs[j].receiver_text {
                    group.push(j);
                }
            }

            if group.len() < 2 {
                continue;
            }

            // RuboCop's uniq_items: keep only items whose receive_msg does NOT
            // appear on a different line within the group. If :foo appears on
            // lines 2 and 4, both are excluded. Items with unique messages stay.
            let uniq_indices: Vec<usize> = group
                .iter()
                .copied()
                .filter(|&idx| {
                    !group.iter().any(|&other| {
                        stubs[idx].receive_msg == stubs[other].receive_msg
                            && stubs[idx].line != stubs[other].line
                    })
                })
                .collect();

            if uniq_indices.len() < 2 {
                // Mark all as processed so we don't re-check
                for &idx in &group {
                    processed[idx] = true;
                }
                continue;
            }

            // RuboCop's repeated_lines logic: for each item, the "repeated lines"
            // are lines of OTHER unique items. If all unique items are on the same
            // line, repeated_lines is empty for each, and no offense is reported.
            let distinct_lines: std::collections::HashSet<usize> =
                uniq_indices.iter().map(|&idx| stubs[idx].line).collect();
            if distinct_lines.len() < 2 {
                for &idx in &group {
                    processed[idx] = true;
                }
                continue;
            }

            for &idx in &group {
                processed[idx] = true;
            }

            // Only report offenses for the unique items
            for &idx in &uniq_indices {
                let (line, column) = self.source.offset_to_line_col(stubs[idx].offset);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Use `receive_messages` instead of multiple stubs.".to_string(),
                ));
            }
        }
    }
}

impl<'pr> Visit<'pr> for ReceiveMessagesVisitor<'_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let skip_begin_body = if self.pending_begin_body > 0 {
            self.pending_begin_body -= 1;
            true
        } else {
            false
        };

        if !skip_begin_body {
            self.check_statements(node);
        }
        ruby_prism::visit_statements_node(self, node);
    }

    /// Prism uses `BeginNode` for both explicit `begin...end` blocks and
    /// handled `begin` bodies with `rescue`/`ensure`, but RuboCop only skips
    /// the pure explicit `begin...end` (`kwbegin`) form.
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        let is_pure_begin = node.rescue_clause().is_none()
            && node.ensure_clause().is_none()
            && node.else_clause().is_none();
        if is_pure_begin {
            self.pending_begin_body += 1;
        }
        ruby_prism::visit_begin_node(self, node);
    }
}

fn extract_allow_receive_info(
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
) -> Option<StubInfo> {
    // RuboCop node pattern:
    // (send (send nil? :allow ...) :to
    //   (send (send nil? :receive (sym _)) :and_return !#heredoc_or_splat?))
    let to_call = node.as_call_node()?;

    if to_call.name().as_slice() != b"to" || to_call.block().is_some() {
        return None;
    }

    // Receiver must be bare allow(...)
    let allow_call = to_call.receiver()?.as_call_node()?;
    if allow_call.name().as_slice() != b"allow" || allow_call.receiver().is_some() {
        return None;
    }

    // Get receiver text
    let allow_args = allow_call.arguments()?;
    let allow_arg_list: Vec<_> = allow_args.arguments().iter().collect();
    if allow_arg_list.is_empty() {
        return None;
    }
    let recv_loc = allow_arg_list[0].location();
    let receiver_text = source
        .byte_slice(recv_loc.start_offset(), recv_loc.end_offset(), "")
        .to_string();

    // Get the argument chain: receive(:y).and_return(z)
    let to_args = to_call.arguments()?;
    let to_arg_list: Vec<_> = to_args.arguments().iter().collect();
    if to_arg_list.len() != 1 {
        return None;
    }

    // Must be direct .and_return(...) call as the only `to` argument.
    let and_return_call = to_arg_list[0].as_call_node()?;
    if and_return_call.name().as_slice() != b"and_return" || and_return_call.block().is_some() {
        return None;
    }

    // and_return receiver must be direct bare receive(:symbol)
    let receive_call = and_return_call.receiver()?.as_call_node()?;
    if receive_call.name().as_slice() != b"receive"
        || receive_call.receiver().is_some()
        || receive_call.block().is_some()
    {
        return None;
    }

    let receive_args = receive_call.arguments()?;
    let receive_arg_list: Vec<_> = receive_args.arguments().iter().collect();
    if receive_arg_list.len() != 1 {
        return None;
    }
    let receive_symbol = receive_arg_list[0].as_symbol_node()?;

    // and_return must have exactly one non-heredoc/non-splat arg.
    let and_return_args = and_return_call.arguments()?;
    let and_return_arg_list: Vec<_> = and_return_args.arguments().iter().collect();
    if and_return_arg_list.len() != 1 || heredoc_or_splat(&and_return_arg_list[0]) {
        return None;
    }

    let stmt_loc = node.location();
    let msg_loc = receive_symbol.location();
    let receive_msg = source
        .byte_slice(msg_loc.start_offset(), msg_loc.end_offset(), "")
        .to_string();
    let (line, _) = source.offset_to_line_col(stmt_loc.start_offset());

    Some(StubInfo {
        receiver_text,
        receive_msg,
        offset: stmt_loc.start_offset(),
        line,
    })
}

fn heredoc_or_splat(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_splat_node().is_some() {
        return true;
    }

    if let Some(string) = node.as_string_node() {
        return string
            .opening_loc()
            .is_some_and(|opening| opening.as_slice().starts_with(b"<<"));
    }

    if let Some(string) = node.as_interpolated_string_node() {
        return string
            .opening_loc()
            .is_some_and(|opening| opening.as_slice().starts_with(b"<<"));
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReceiveMessages, "cops/rspec/receive_messages");
}
