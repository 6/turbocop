use ruby_prism::Visit;

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
            in_where_args: 0,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct PluckIdVisitor<'a, 'src> {
    cop: &'a PluckId,
    source: &'src SourceFile,
    /// Depth counter for being inside where/rewhere argument subtrees.
    in_where_args: usize,
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
            if pk_call.name().as_slice() == b"primary_key" && pk_call.receiver().is_none() {
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
}

impl<'pr> Visit<'pr> for PluckIdVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        // Check if this is a pluck(:id) or pluck(primary_key) call
        if self.in_where_args == 0 {
            if let Some(message) = self.check_pluck_call(node) {
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
        }

        // If this is a where/rewhere/where.not call, mark arguments as in-where
        if Self::is_where_call(node) {
            // Visit receiver normally (pluck as receiver of where should still be flagged)
            if let Some(recv) = node.receiver() {
                self.visit(&recv);
            }
            // Visit arguments with in_where_args incremented
            if let Some(args) = node.arguments() {
                self.in_where_args += 1;
                self.visit_arguments_node(&args);
                self.in_where_args -= 1;
            }
            // Visit block if any
            if let Some(block) = node.block() {
                self.visit(&block);
            }
            return;
        }

        // Default: visit all children
        ruby_prism::visit_call_node(self, node);
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
