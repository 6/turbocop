use std::collections::HashSet;
use std::sync::Mutex;

use crate::cop::shared::literal_predicates;
use crate::cop::variable_force::{self, Scope, VariableTable};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/InfiniteLoop
///
/// ## Investigation findings
///
/// ### Round 1
///
/// FP=317 root cause: RuboCop uses VariableForce to track local variable
/// assignments and references. It skips the offense when a local variable is
/// first assigned inside the `while true`/`until false` loop body and then
/// referenced after the loop ends — because converting to `loop do` would
/// create a block scope that hides the variable. nitrocop was not implementing
/// this variable scoping exemption at all.
///
/// FN=19 root cause: nitrocop only matched `true`/`false` literals as
/// conditions. RuboCop's `truthy_literal?` also matches integer, float, array,
/// and hash literals; `falsey_literal?` also matches `nil`.
///
/// Fix: switched from `check_node` to `check_source` with a visitor that
/// collects local variable writes inside loop bodies and reads after the loop,
/// implementing the variable scoping exemption. Also added truthy/falsey
/// literal detection for integers, floats, arrays, hashes, and nil.
///
/// Additional FN reduction: nested `while true` / `until false` loops under
/// Prism statement-bearing nodes like `if`, `else`, and `begin` were still
/// missed because the visitor only called `check_statements` from a small
/// whitelist of parent node types. Prism already visits every statement list
/// through `StatementsNode`, so this cop now checks each `StatementsNode`
/// exactly once and evaluates the scoping exemption against the enclosing
/// lexical scope instead of only immediate sibling statements.
///
/// ### Round 2 (FP=2, FN=37)
///
/// Three root causes found and fixed:
///
/// 1. **FN from block-local variable collisions (17 cases)**: `LvarWriteCollector`
///    entered block bodies (`do..end`), collecting block-local variables. When
///    a same-named variable appeared elsewhere in the method, `has_lvar_read_after`
///    falsely triggered the scoping exemption. Fix: stop `LvarWriteCollector`
///    and `ScopedLvarWriteChecker` at `BlockNode` boundaries.
///
/// 2. **FN from while-as-expression (20 natalie cases)**: `check_statements`
///    only looked at direct `StatementsNode` children, missing `while true`
///    nested inside assignments (`a = while true; break; end`). Fix: switched
///    to `visit_while_node`/`visit_until_node` visitor methods.
///
/// 3. **FP from missing operator write support (2 cases)**: Compound assignments
///    like `offset += 1` (`LocalVariableOperatorWriteNode`) were not tracked
///    by any variable collector. This caused the scoping exemption to miss
///    parameter modifications inside loops. Fix: added support for
///    `LocalVariableOperatorWriteNode`, `LocalVariableAndWriteNode`, and
///    `LocalVariableOrWriteNode` in all variable collectors.
///
/// Additionally, `BlockNode` now pushes onto `scope_stack` so variable scoping
/// is evaluated against the enclosing block scope (not the entire method body),
/// preventing false matches with same-named variables in sibling blocks.
///
/// ### Round 3 (0 FP, 5 FN)
///
/// Three remaining FN root causes were found and fixed:
///
/// 1. **Nested block ancestor locals were treated as out of scope**: when a
///    `while true` lived inside a block, the scoping exemption only looked for
///    prior assignments inside that block body. This missed outer locals that
///    are still visible inside nested blocks (`now` before `mutex.synchronize`,
///    `matched` before an inner `each do`). Fix: track closure-capable scope
///    chains and search visible ancestor scopes for earlier assignments.
///
/// 2. **Read-after checks crossed block boundaries**: later reads inside nested
///    blocks/lambdas were treated as reads of loop-local variables, even when a
///    block parameter shadowed the name (for example a later `|event|`). Fix:
///    stop `ScopedLvarReadChecker` at `BlockNode` boundaries, matching the write
///    collectors' lexical-scope behavior.
///
/// 3. **Truthy literal detection still missed string-like conditions**:
///    RuboCop's `truthy_literal?` includes string, xstring, symbol, range,
///    regexp, rational, and imaginary literals. This cop was still missing the
///    backtick/xstring corpus case. Fix: extend `is_truthy_literal` to cover the
///    same literal families that Prism exposes for this cop.
///
/// ### VariableForce migration
///
/// Migrated to use the shared VariableForce engine. The cop now uses a hybrid
/// approach: `check_source` finds all `while true`/`until false` loops and
/// records their offset ranges. The VF `before_leaving_scope` hook checks each
/// loop in the leaving scope against VF variable data to determine the scoping
/// exemption (whether converting to `loop do` would break variable visibility).
/// This replaces the previous 544-line standalone AST visitor with ~200 lines
/// of VF-integrated code.
pub struct InfiniteLoop {
    /// Potential offenses found by `check_source`. Each entry is a loop that
    /// has a truthy/falsey condition. The VF hook will filter out loops where
    /// the conversion would break variable scoping.
    loops: Mutex<Vec<LoopInfo>>,
    /// Loop offsets that have already been processed by an inner scope's
    /// `before_leaving_scope`. Prevents duplicate emissions from outer scopes.
    processed: Mutex<HashSet<usize>>,
}

impl InfiniteLoop {
    pub fn new() -> Self {
        Self {
            loops: Mutex::new(Vec::new()),
            processed: Mutex::new(HashSet::new()),
        }
    }
}

impl Default for InfiniteLoop {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a `while true`/`until false` loop found during check_source.
struct LoopInfo {
    /// Byte offset of the keyword (`while`/`until`).
    keyword_offset: usize,
    /// Byte offset of the loop node's start.
    loop_start: usize,
    /// Byte offset of the loop node's end.
    loop_end: usize,
    /// Byte offset of the loop body start (statements inside the loop).
    body_start: usize,
    /// Byte offset of the loop body end.
    body_end: usize,
}

impl Cop for InfiniteLoop {
    fn name(&self) -> &'static str {
        "Style/InfiniteLoop"
    }

    fn check_source(
        &self,
        _source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &CodeMap,
        _config: &CopConfig,
        _diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut collector = LoopCollector { loops: Vec::new() };
        collector.visit(&parse_result.node());
        *self.loops.lock().unwrap() = collector.loops;
        self.processed.lock().unwrap().clear();
    }

    fn as_variable_force_consumer(&self) -> Option<&dyn variable_force::VariableForceConsumer> {
        Some(self)
    }
}

impl variable_force::VariableForceConsumer for InfiniteLoop {
    fn before_leaving_scope(
        &self,
        scope: &Scope,
        variable_table: &VariableTable,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let loops = self.loops.lock().unwrap();
        let mut processed = self.processed.lock().unwrap();

        for loop_info in loops.iter() {
            // Skip loops already handled by an inner scope.
            if processed.contains(&loop_info.loop_start) {
                continue;
            }

            // The loop must be inside this scope's range.
            if loop_info.loop_start < scope.node_start_offset
                || loop_info.loop_end > scope.node_end_offset
            {
                continue;
            }

            // Mark as processed so outer scopes don't re-emit.
            processed.insert(loop_info.loop_start);

            if would_break_scoping(scope, variable_table, loop_info) {
                continue;
            }

            let (line, column) = source.offset_to_line_col(loop_info.keyword_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Use `Kernel#loop` for infinite loops.".to_string(),
            ));
        }
    }
}

/// Check if converting a while/until loop to `loop do` would break variable scoping.
///
/// A variable is "loop-local" if:
/// 1. Its first assignment is inside the loop body (no prior assignment exists)
/// 2. It is referenced after the loop ends
///
/// If such a variable exists, `loop do` would hide it (block scoping), so we
/// suppress the offense.
fn would_break_scoping(
    scope: &Scope,
    variable_table: &VariableTable,
    loop_info: &LoopInfo,
) -> bool {
    // Check variables in the current scope and all accessible outer scopes
    // (walking the closure chain). This handles variables declared in an
    // outer def/block that are visible inside nested blocks containing
    // the while loop.
    for accessible_scope in variable_table.accessible_scopes() {
        // Only check scopes that contain the loop
        if loop_info.loop_start < accessible_scope.node_start_offset
            || loop_info.loop_end > accessible_scope.node_end_offset
        {
            continue;
        }
        for variable in accessible_scope.variables.values() {
            if check_variable_breaks_scoping(variable, loop_info) {
                return true;
            }
        }
    }

    // Also check the current scope being left (it may not be on the stack
    // yet if `accessible_scopes` walks from the current top). Since
    // `before_leaving_scope` fires while the scope is still on the stack,
    // `accessible_scopes()` should include it. But as a safety net, also
    // check the passed-in scope directly.
    if loop_info.loop_start >= scope.node_start_offset
        && loop_info.loop_end <= scope.node_end_offset
    {
        for variable in scope.variables.values() {
            if check_variable_breaks_scoping(variable, loop_info) {
                return true;
            }
        }
    }

    false
}

/// Check if a single variable's assignment/reference pattern would break
/// scoping if the loop were converted to `loop do`.
fn check_variable_breaks_scoping(
    variable: &variable_force::Variable,
    loop_info: &LoopInfo,
) -> bool {
    // Check if the variable has ANY assignment inside the loop body
    let has_assignment_inside = variable
        .assignments
        .iter()
        .any(|a| a.node_offset >= loop_info.body_start && a.node_offset < loop_info.body_end);

    if !has_assignment_inside {
        return false;
    }

    // Check if there's a prior non-argument assignment (before the loop starts).
    // If there is, the variable already exists in the outer scope, so
    // converting to `loop do` won't hide it.
    //
    // Note: arguments are NOT excluded here. RuboCop treats `offset += 1`
    // inside a loop the same as any other assignment — if `offset` is only
    // declared as a parameter (no prior non-param assignment) and is referenced
    // after the loop, the offense is suppressed. This is conservative but
    // matches RuboCop's behavior.
    let has_assignment_before = variable
        .assignments
        .iter()
        .any(|a| a.node_offset < loop_info.loop_start);

    if has_assignment_before {
        return false;
    }

    // The variable's first assignment is inside the loop body.
    // Check if it's referenced after the loop ends.
    variable
        .references
        .iter()
        .any(|r| r.node_offset >= loop_info.loop_end)
}

// ── Loop collector ────────────────────────────────────────────────────

fn is_truthy_literal(node: &ruby_prism::Node<'_>) -> bool {
    literal_predicates::is_truthy_literal(node)
}

fn is_falsey_literal(node: &ruby_prism::Node<'_>) -> bool {
    literal_predicates::is_falsey_literal(node)
}

/// AST visitor that finds all `while true`/`until false` loops and records
/// their offset ranges for later VF-based scoping analysis.
struct LoopCollector {
    loops: Vec<LoopInfo>,
}

impl<'pr> Visit<'pr> for LoopCollector {
    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        if is_truthy_literal(&node.predicate()) {
            if let Some(stmts) = node.statements() {
                self.loops.push(LoopInfo {
                    keyword_offset: node.keyword_loc().start_offset(),
                    loop_start: node.location().start_offset(),
                    loop_end: node.location().end_offset(),
                    body_start: stmts.location().start_offset(),
                    body_end: stmts.location().end_offset(),
                });
            } else {
                // No body — still record with zero-size body range
                let loc = node.location();
                self.loops.push(LoopInfo {
                    keyword_offset: node.keyword_loc().start_offset(),
                    loop_start: loc.start_offset(),
                    loop_end: loc.end_offset(),
                    body_start: loc.end_offset(),
                    body_end: loc.end_offset(),
                });
            }
        }
        ruby_prism::visit_while_node(self, node);
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        if is_falsey_literal(&node.predicate()) {
            if let Some(stmts) = node.statements() {
                self.loops.push(LoopInfo {
                    keyword_offset: node.keyword_loc().start_offset(),
                    loop_start: node.location().start_offset(),
                    loop_end: node.location().end_offset(),
                    body_start: stmts.location().start_offset(),
                    body_end: stmts.location().end_offset(),
                });
            } else {
                let loc = node.location();
                self.loops.push(LoopInfo {
                    keyword_offset: node.keyword_loc().start_offset(),
                    loop_start: loc.start_offset(),
                    loop_end: loc.end_offset(),
                    body_start: loc.end_offset(),
                    body_end: loc.end_offset(),
                });
            }
        }
        ruby_prism::visit_until_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InfiniteLoop::new(), "cops/style/infinite_loop");
}
