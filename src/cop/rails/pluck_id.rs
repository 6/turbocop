use ruby_prism::Visit;

use crate::cop::shared::method_dispatch_predicates;
use crate::cop::{Cop, CopConfig, EnabledState};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Enforces the use of `ids` over `pluck(:id)` and `pluck(primary_key)`.
///
/// ## Investigation findings (2026-03-08)
///
/// **FP=502:** Missing `in_where?` exemption. The vendor RuboCop cop skips `pluck(:id)`
/// when used as an argument inside a `where`/`rewhere` call (or `where.not`). For example,
/// `Post.where(user_id: User.pluck(:id))` should NOT be flagged. Switched from `check_node`
/// to `check_source` with a visitor that tracks `in_where_args` depth to implement this.
///
/// **FN=354:** Missing `pluck(primary_key)` detection. The vendor NodePattern also matches
/// `(send nil? :primary_key)` — a bare `primary_key` method call with no receiver. Added
/// detection for CallNode with name `primary_key` and no receiver as an argument to `pluck`.
///
/// Also fixed offense range to match vendor behavior: offense spans from the method name
/// (`pluck`) to the end of the call, not the entire call chain including receiver.
///
/// ## Investigation findings (2026-03-16)
///
/// **FN=11 across 9 repos:** Root cause identified: `in_where_args` depth counter was too
/// broad. It exempted ALL descendants of `where`/`rewhere` arguments from flagging, but the
/// vendor RuboCop `in_where?` only exempts `pluck` calls whose **immediate call ancestor** is
/// `where`/`rewhere`/`where.not`.
///
/// Examples that RuboCop flags (but old nitrocop missed):
/// - `where(id: [a] + pluck(:id))` — first ancestor is `+`, not `where`
/// - `where(id: map { pluck(:id) })` — first ancestor is `map` (block context)
/// - `where(arel.in(pluck(:id)))` — first ancestor is `in`, not `where`
/// - `where(id: pluck(:id) | [x])` — first ancestor is `|`
///
/// Fixed by replacing `in_where_args` counter with an `immediate_call_ancestor` stack that
/// tracks the nearest enclosing call at each point in the tree. When we encounter a `pluck`
/// node, we check whether its immediate call ancestor is `where`/`rewhere`/`where.not` (and
/// the pluck is not the receiver of that where call). This exactly mirrors vendor `in_where?`.
pub struct PluckId;

impl Cop for PluckId {
    fn name(&self) -> &'static str {
        "Rails/PluckId"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if config.enabled != EnabledState::True {
            return;
        }

        let mut visitor = PluckIdVisitor {
            cop: self,
            source,
            // Stack of (is_where_call, call_start_offset) for the enclosing calls.
            // We only need to know whether the immediate parent call is a where/rewhere/where.not
            // and what the receiver start offset is (so we can exclude pluck-as-receiver).
            parent_call_stack: Vec::new(),
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

/// Information about an enclosing call node pushed onto the parent stack.
#[derive(Clone, Copy)]
struct ParentCallInfo {
    /// Whether this call is where/rewhere/where.not.
    is_where: bool,
    /// Start offset of the receiver of this call (if any), used to detect
    /// `pluck` as the receiver of `where` (which should NOT be exempted).
    receiver_start: Option<usize>,
    /// End offset of the receiver of this call (if any).
    receiver_end: Option<usize>,
}

struct PluckIdVisitor<'a, 'src> {
    cop: &'a PluckId,
    source: &'src SourceFile,
    /// Stack of parent call info. Top of stack = immediate enclosing call.
    parent_call_stack: Vec<ParentCallInfo>,
    diagnostics: Vec<Diagnostic>,
}

impl PluckIdVisitor<'_, '_> {
    /// Check if a call node is a pluck(:id) or pluck(primary_key) call.
    /// Returns the appropriate message if it matches, None otherwise.
    fn check_pluck_call(&self, call: &ruby_prism::CallNode<'_>) -> Option<&'static str> {
        if call.name().as_slice() != b"pluck" {
            return None;
        }

        let args = call.arguments()?;
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return None;
        }

        // Check for pluck(:id)
        if let Some(sym) = arg_list[0].as_symbol_node() {
            if sym.unescaped() == b"id" {
                return Some("Use `ids` instead of `pluck(:id)`.");
            }
        }

        // Check for pluck(primary_key) — bare method call with no receiver
        if let Some(pk_call) = arg_list[0].as_call_node() {
            if method_dispatch_predicates::is_command(&pk_call, b"primary_key") {
                return Some("Use `ids` instead of `pluck(primary_key)`.");
            }
        }

        None
    }

    /// Check if a call is `where`, `rewhere`, or a `.not` chained on `where`/`rewhere`.
    fn is_where_call(call: &ruby_prism::CallNode<'_>) -> bool {
        let name = call.name().as_slice();
        if name == b"where" || name == b"rewhere" {
            return true;
        }
        // Handle where(...).not(...) — if this is `.not` and its receiver is where/rewhere
        if name == b"not" {
            if let Some(recv) = call.receiver() {
                if let Some(recv_call) = recv.as_call_node() {
                    let recv_name = recv_call.name().as_slice();
                    if recv_name == b"where" || recv_name == b"rewhere" {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Mirrors the vendor `in_where?` logic: returns true if the current pluck call
    /// should be exempted because its immediate call ancestor is where/rewhere/where.not
    /// and the pluck is not the receiver of that where call.
    fn pluck_is_in_where(&self, pluck_start: usize) -> bool {
        if let Some(parent) = self.parent_call_stack.last() {
            if !parent.is_where {
                return false;
            }
            // If the pluck call is the receiver of the where call, it should NOT be exempted.
            // We detect this by checking if the pluck's start offset matches the receiver range.
            if let (Some(recv_start), Some(recv_end)) = (parent.receiver_start, parent.receiver_end)
            {
                if pluck_start >= recv_start && pluck_start < recv_end {
                    return false; // pluck is the receiver of where — flag it
                }
            }
            return true;
        }
        false
    }
}

impl<'pr> Visit<'pr> for PluckIdVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this is a pluck(:id) or pluck(primary_key) call
        if let Some(message) = self.check_pluck_call(node) {
            let call_start = node.location().start_offset();
            if !self.pluck_is_in_where(call_start) {
                // Offense range: from the method name to end of call
                let msg_loc = node.message_loc();
                let call_loc = node.location();
                let start_offset = match msg_loc {
                    Some(loc) => loc.start_offset(),
                    None => call_loc.start_offset(),
                };
                let (line, column) = self.source.offset_to_line_col(start_offset);
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    message.to_string(),
                ));
            }
            // No children to visit for a valid pluck(:id) call (the arg is :id or primary_key)
            return;
        }

        // Push parent info for this call onto the stack before visiting children.
        // All children will see this call as their immediate parent.
        let receiver_start = node.receiver().map(|r| r.location().start_offset());
        let receiver_end = node.receiver().map(|r| r.location().end_offset());
        let info = ParentCallInfo {
            is_where: Self::is_where_call(node),
            receiver_start,
            receiver_end,
        };
        self.parent_call_stack.push(info);

        // Visit receiver — receiver of a where/rewhere can itself contain pluck calls
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        // Visit arguments
        if let Some(args) = node.arguments() {
            self.visit_arguments_node(&args);
        }
        // Visit block if any
        if let Some(block) = node.block() {
            self.visit(&block);
        }

        self.parent_call_stack.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_config() -> CopConfig {
        CopConfig {
            enabled: EnabledState::True,
            ..CopConfig::default()
        }
    }

    #[test]
    fn offense_fixture() {
        crate::testutil::assert_cop_offenses_full_with_config(
            &PluckId,
            include_bytes!("../../../tests/fixtures/cops/rails/pluck_id/offense.rb"),
            enabled_config(),
        );
    }

    #[test]
    fn no_offense_fixture() {
        crate::testutil::assert_cop_no_offenses_full_with_config(
            &PluckId,
            include_bytes!("../../../tests/fixtures/cops/rails/pluck_id/no_offense.rb"),
            enabled_config(),
        );
    }
}
