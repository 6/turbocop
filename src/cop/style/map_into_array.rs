use std::sync::Mutex;

use crate::cop::variable_force::{self, Scope, VariableTable};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for usages of `each` with `<<`, `push`, or `append` which
/// can be replaced by `map`.
///
/// ## Investigation Notes (2026-03-18)
///
/// **FP root causes fixed:**
/// - Safe navigation `&.each` (e.g., `opts[:key]&.each { |x| arr << x }`) was not
///   excluded. RuboCop's NodePattern uses `(send ...)` which only matches regular
///   method calls, not `csend` (safe navigation). Fixed by checking
///   `call.call_operator()` for `&.`.
/// - `LocalVariableOperatorWriteNode` (e.g., `arr += other`) between array init and
///   each loop was not detected by `references_variable`, causing the cop to miss
///   that the array was modified. Same issue with `LocalVariableOrWriteNode` and
///   `LocalVariableAndWriteNode`. Fixed by adding these node types to both the
///   backwards assignment search and the `references_variable` helper.
///
/// **FN root causes fixed:**
/// - `Array.new` and `Array[]` as empty array initializers were not recognized.
///   RuboCop accepts `Array.new`, `Array.new([])`, `Array[]`, and `Array([])`.
///   Added detection for `Array.new` (no args or empty array arg) and `Array[]`
///   (no args) as `CallNode` patterns.
///
/// ## Investigation Notes (2026-03-19)
///
/// **FP root cause (1 FP):**
/// - `binding` inside the each block body implicitly captures all local variables
///   in scope (including the destination array variable). RuboCop's `VariableForce`
///   counts `binding` calls as implicit references, so `dest_var.references.one?`
///   returns false and the cop doesn't flag it. Fixed by checking for `binding`
///   calls inside the each block body and skipping if found.
///
/// **FN root causes (7 FN):**
/// - The `[].tap { |dest| src.each { |e| dest << expr } }` pattern was not
///   handled at all. RuboCop supports this via `empty_array_tap` node matcher.
///   Fixed by adding `visit_block_node` to detect `[].tap` blocks where the
///   only body statement is an `each` with push into the tap block parameter.
///   The tap block must contain only the each call (no other statements).
///
/// ## Migration to VariableForce (2026-04-02)
///
/// Migrated from a standalone AST visitor with manual variable analysis to a
/// hybrid check_source + VariableForce approach. The pattern matching (detecting
/// `each` + `<<`/`push`/`append` with `var = []` init, and `[].tap` patterns)
/// remains in `check_source`. The variable analysis (binding detection,
/// intermediate reference checking, operator assignment detection) is now
/// handled by VariableForce's `before_leaving_scope` hook, which validates
/// candidates using VF's complete variable lifetime data. This removed
/// ~120 lines of manual `contains_binding_call`, `references_variable`,
/// and `is_local_var_*_write` helpers.
pub struct MapIntoArray {
    /// Candidate offenses found by `check_source` pattern matching.
    /// Validated by `before_leaving_scope` using VF variable data.
    candidates: Mutex<Vec<Candidate>>,
}

impl MapIntoArray {
    pub fn new() -> Self {
        Self {
            candidates: Mutex::new(Vec::new()),
        }
    }
}

impl Default for MapIntoArray {
    fn default() -> Self {
        Self::new()
    }
}

/// A candidate offense found during pattern matching in `check_source`.
/// Needs VF validation before being emitted as a diagnostic.
#[derive(Debug)]
struct Candidate {
    /// Name of the destination variable (e.g., `dest` in `dest << x`).
    var_name: Vec<u8>,
    /// Byte offset of the `var = []` assignment.
    init_offset: usize,
    /// Byte offset of the `each` call node.
    each_offset: usize,
    /// Source line of the `each` call (for diagnostic).
    line: usize,
    /// Source column of the `each` call (for diagnostic).
    column: usize,
    /// Kind of candidate pattern.
    kind: CandidateKind,
}

#[derive(Debug)]
enum CandidateKind {
    /// `dest = []; src.each { |x| dest << expr }`
    /// Needs: no refs between init and each, no binding in block body.
    EachPush {
        /// Byte offset of the each block body start (for binding check).
        block_body_start: usize,
        /// Byte offset of the each block body end (for binding check).
        block_body_end: usize,
    },
    /// `[].tap { |dest| src.each { |e| dest << expr } }`
    /// The dest var is a block param in the tap block scope.
    /// Needs: no binding in the each block body.
    TapEachPush {
        /// Byte offset of the tap block parameter declaration (to match the right scope).
        param_offset: usize,
        /// Byte offset of the each block body start (for binding check).
        block_body_start: usize,
        /// Byte offset of the each block body end (for binding check).
        block_body_end: usize,
    },
}

/// Check if a node is an empty array expression: `[]`, `Array.new`, `Array.new([])`,
/// `Array[]`, or `Array([])`.
fn is_empty_array_value(value: &ruby_prism::Node<'_>) -> bool {
    // Literal empty array: `[]`
    if let Some(arr) = value.as_array_node() {
        return arr.elements().iter().count() == 0;
    }
    // Call-based patterns: `Array.new`, `Array.new([])`, `Array[]`, `Array([])`
    if let Some(call) = value.as_call_node() {
        let method = call.name().as_slice();
        if let Some(receiver) = call.receiver() {
            // `Array.new` or `Array.new([])`  or  `Array[]`
            let is_array_const = receiver
                .as_constant_read_node()
                .is_some_and(|c| c.name().as_slice() == b"Array")
                || receiver
                    .as_constant_path_node()
                    .is_some_and(|cp| cp.name().is_some_and(|n| n.as_slice() == b"Array"));
            if is_array_const {
                if method == b"new" {
                    // Array.new or Array.new([])
                    if call.arguments().is_none() {
                        return true;
                    }
                    if let Some(args) = call.arguments() {
                        let arg_list: Vec<_> = args.arguments().iter().collect();
                        if arg_list.len() == 1 {
                            if let Some(arr) = arg_list[0].as_array_node() {
                                return arr.elements().iter().count() == 0;
                            }
                        }
                    }
                } else if method == b"[]" {
                    // Array[]
                    return call.arguments().is_none();
                }
            }
        } else {
            // `Array([])` — this is a Kernel method call with no receiver
            if method == b"Array" {
                if let Some(args) = call.arguments() {
                    let arg_list: Vec<_> = args.arguments().iter().collect();
                    if arg_list.len() == 1 {
                        if let Some(arr) = arg_list[0].as_array_node() {
                            return arr.elements().iter().count() == 0;
                        }
                    }
                }
            }
        }
    }
    false
}

impl Cop for MapIntoArray {
    fn name(&self) -> &'static str {
        "Style/MapIntoArray"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        _diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut finder = CandidateFinder {
            source,
            candidates: Vec::new(),
        };
        finder.visit(&parse_result.node());
        *self.candidates.lock().unwrap() = finder.candidates;
    }

    fn as_variable_force_consumer(&self) -> Option<&dyn variable_force::VariableForceConsumer> {
        Some(self)
    }
}

impl variable_force::VariableForceConsumer for MapIntoArray {
    fn before_leaving_scope(
        &self,
        scope: &Scope,
        _variable_table: &VariableTable,
        source: &SourceFile,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let candidates = self.candidates.lock().unwrap();
        for candidate in candidates.iter() {
            // Check if the candidate's variable exists in this scope
            let var = match scope.variables.get(&candidate.var_name) {
                Some(v) => v,
                None => continue,
            };

            match &candidate.kind {
                CandidateKind::EachPush {
                    block_body_start,
                    block_body_end,
                } => {
                    // Check that the variable's init assignment is within this scope
                    // and matches the expected offset
                    let has_init = var
                        .assignments
                        .iter()
                        .any(|a| a.node_offset == candidate.init_offset && !a.is_operator());
                    if !has_init {
                        continue;
                    }

                    // Check no explicit references between init and each offsets
                    let has_intermediate_ref = var.references.iter().any(|r| {
                        r.explicit
                            && r.node_offset > candidate.init_offset
                            && r.node_offset < candidate.each_offset
                    });
                    if has_intermediate_ref {
                        continue;
                    }

                    // Check no operator/or/and assignments between init and each
                    let has_intermediate_operator_assign = var.assignments.iter().any(|a| {
                        a.is_operator()
                            && a.node_offset > candidate.init_offset
                            && a.node_offset < candidate.each_offset
                    });
                    if has_intermediate_operator_assign {
                        continue;
                    }

                    // Check no implicit references (from binding) within the block body.
                    // VF's engine adds implicit references when it encounters `binding`
                    // calls, with the offset of the `binding` call site.
                    let has_binding_in_block = var.references.iter().any(|r| {
                        !r.explicit
                            && r.node_offset >= *block_body_start
                            && r.node_offset <= *block_body_end
                    });
                    if has_binding_in_block {
                        continue;
                    }

                    diagnostics.push(self.diagnostic(
                        source,
                        candidate.line,
                        candidate.column,
                        "Use `map` instead of `each` to map elements into an array.".to_string(),
                    ));
                }
                CandidateKind::TapEachPush {
                    param_offset,
                    block_body_start,
                    block_body_end,
                } => {
                    // Verify this is the right scope: the variable's declaration
                    // must match the tap block parameter offset.
                    if var.declaration_offset != *param_offset {
                        continue;
                    }

                    // For tap pattern, dest var is a block parameter.
                    // Only need to check for binding in the each block body.
                    let has_binding_in_block = var.references.iter().any(|r| {
                        !r.explicit
                            && r.node_offset >= *block_body_start
                            && r.node_offset <= *block_body_end
                    });
                    if has_binding_in_block {
                        continue;
                    }

                    diagnostics.push(self.diagnostic(
                        source,
                        candidate.line,
                        candidate.column,
                        "Use `map` instead of `each` to map elements into an array.".to_string(),
                    ));
                }
            }
        }
    }
}

// ── Pattern-matching AST visitor ─────────────────────────────────────

struct CandidateFinder<'a> {
    source: &'a SourceFile,
    candidates: Vec<Candidate>,
}

impl CandidateFinder<'_> {
    /// Check if a statements node contains:
    ///   dest = []
    ///   ...each { |x| dest << expr }
    /// Pattern: look at pairs of siblings in a statements block.
    fn check_statements(&mut self, stmts: &[ruby_prism::Node<'_>]) {
        for (i, stmt) in stmts.iter().enumerate() {
            // Check if this is a `collection.each { |x| var << expr }` pattern
            let call = match stmt.as_call_node() {
                Some(c) => c,
                None => continue,
            };

            if call.name().as_slice() != b"each" {
                continue;
            }
            if call.receiver().is_none() {
                continue;
            }
            // Skip safe navigation `&.each` — RuboCop only matches `send`, not `csend`
            if call
                .call_operator_loc()
                .is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.")
            {
                continue;
            }
            // each must have no arguments
            if call.arguments().is_some() {
                continue;
            }

            let block = match call.block() {
                Some(b) => b,
                None => continue,
            };
            let block_node = match block.as_block_node() {
                Some(b) => b,
                None => continue,
            };
            let body = match block_node.body() {
                Some(b) => b,
                None => continue,
            };
            let body_stmts = match body.as_statements_node() {
                Some(s) => s,
                None => continue,
            };
            let body_nodes: Vec<_> = body_stmts.body().iter().collect();
            if body_nodes.len() != 1 {
                continue;
            }

            // Check for var << expr or var.push(expr) or var.append(expr)
            let push_call = match body_nodes[0].as_call_node() {
                Some(c) => c,
                None => continue,
            };
            let push_method = push_call.name().as_slice();
            if push_method != b"<<" && push_method != b"push" && push_method != b"append" {
                continue;
            }

            // Receiver must be a local variable
            let push_receiver = match push_call.receiver() {
                Some(r) => r,
                None => continue,
            };
            let lvar = match push_receiver.as_local_variable_read_node() {
                Some(l) => l,
                None => continue,
            };

            let var_name = lvar.name();

            // Check that the push argument is suitable (not a splat, etc.)
            if let Some(args) = push_call.arguments() {
                let arg_list: Vec<_> = args.arguments().iter().collect();
                if arg_list.len() != 1 {
                    continue;
                }
                // Skip if argument is a splat
                if arg_list[0].as_splat_node().is_some() {
                    continue;
                }
            } else {
                continue;
            }

            // Now check: is there a preceding `var = []` (or Array.new etc.) in the same scope?
            let mut found_empty_array_init = false;
            let mut init_offset = 0;
            for j in (0..i).rev() {
                // Check plain assignment: `var = expr`
                if let Some(asgn) = stmts[j].as_local_variable_write_node() {
                    if asgn.name().as_slice() == var_name.as_slice() {
                        // Check if the value is an empty array
                        if is_empty_array_value(&asgn.value()) {
                            found_empty_array_init = true;
                            init_offset = asgn.location().start_offset();
                        }
                        break; // found the most recent assignment, stop
                    }
                }
                // Check operator assignments (+=, ||=, &&=) — these mean the var
                // was modified, so any earlier `var = []` is stale.
                if stmts[j]
                    .as_local_variable_operator_write_node()
                    .is_some_and(|n| n.name().as_slice() == var_name.as_slice())
                    || stmts[j]
                        .as_local_variable_or_write_node()
                        .is_some_and(|n| n.name().as_slice() == var_name.as_slice())
                    || stmts[j]
                        .as_local_variable_and_write_node()
                        .is_some_and(|n| n.name().as_slice() == var_name.as_slice())
                {
                    break; // var was modified by operator assignment, stop
                }
            }

            if !found_empty_array_init {
                continue;
            }

            // Receiver of `each` must not be `self`
            if let Some(each_receiver) = call.receiver() {
                if each_receiver.as_self_node().is_some() {
                    continue;
                }
            }

            let loc = call.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());

            // Get block body offsets for binding check
            let body_loc = body.location();
            self.candidates.push(Candidate {
                var_name: var_name.as_slice().to_vec(),
                init_offset,
                each_offset: loc.start_offset(),
                line,
                column,
                kind: CandidateKind::EachPush {
                    block_body_start: body_loc.start_offset(),
                    block_body_end: body_loc.end_offset(),
                },
            });
        }
    }

    /// Check for tap pattern on a call node: `[].tap { |dest| src.each { |e| dest << expr } }`
    fn check_tap_call(&mut self, call: &ruby_prism::CallNode<'_>) {
        // Must be `.tap` with an empty array receiver
        if call.name().as_slice() != b"tap" {
            return;
        }
        if call.arguments().is_some() {
            return;
        }
        // Receiver must be an empty array literal `[]`
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };
        if let Some(arr) = receiver.as_array_node() {
            if arr.elements().iter().count() != 0 {
                return;
            }
        } else {
            return;
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // Block must have exactly one parameter
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return,
        };
        let param_list = match block_params.parameters() {
            Some(pl) => pl,
            None => return,
        };
        let requireds: Vec<_> = param_list.requireds().iter().collect();
        if requireds.len() != 1 {
            return;
        }
        let param_node = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return,
        };
        // Get the block parameter name
        let block_param_name = param_node.name();

        // Block body must have exactly one statement: the each call
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };
        let body_stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };
        let body_nodes: Vec<_> = body_stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return;
        }

        // The single statement must be an `each` call
        let each_call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return,
        };
        if each_call.name().as_slice() != b"each" {
            return;
        }
        if each_call.receiver().is_none() {
            return;
        }
        // Skip safe navigation
        if each_call
            .call_operator_loc()
            .is_some_and(|op: ruby_prism::Location<'_>| op.as_slice() == b"&.")
        {
            return;
        }
        if each_call.arguments().is_some() {
            return;
        }

        // each must have a block
        let each_block = match each_call.block() {
            Some(b) => b,
            None => return,
        };
        let each_block_node = match each_block.as_block_node() {
            Some(b) => b,
            None => return,
        };
        let each_body = match each_block_node.body() {
            Some(b) => b,
            None => return,
        };
        let each_body_stmts = match each_body.as_statements_node() {
            Some(s) => s,
            None => return,
        };
        let each_body_nodes: Vec<_> = each_body_stmts.body().iter().collect();
        if each_body_nodes.len() != 1 {
            return;
        }

        // Check for dest << expr or dest.push(expr) or dest.append(expr)
        let push_call = match each_body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return,
        };
        let push_method = push_call.name().as_slice();
        if push_method != b"<<" && push_method != b"push" && push_method != b"append" {
            return;
        }

        // Push receiver must be the tap block parameter
        let push_receiver = match push_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let lvar = match push_receiver.as_local_variable_read_node() {
            Some(l) => l,
            None => return,
        };
        if lvar.name().as_slice() != block_param_name.as_slice() {
            return;
        }

        // Check push has exactly one non-splat argument
        if let Some(args) = push_call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 1 {
                return;
            }
            if arg_list[0].as_splat_node().is_some() {
                return;
            }
        } else {
            return;
        }

        // Receiver of `each` must not be `self`
        if let Some(each_receiver) = each_call.receiver() {
            if each_receiver.as_self_node().is_some() {
                return;
            }
        }

        // Report offense on the each call
        let loc = each_call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());

        // Get each block body offsets for binding check
        let each_body_loc = each_body.location();
        self.candidates.push(Candidate {
            var_name: block_param_name.as_slice().to_vec(),
            init_offset: 0, // not used for tap pattern
            each_offset: loc.start_offset(),
            line,
            column,
            kind: CandidateKind::TapEachPush {
                param_offset: param_node.location().start_offset(),
                block_body_start: each_body_loc.start_offset(),
                block_body_end: each_body_loc.end_offset(),
            },
        });
    }
}

impl<'pr> Visit<'pr> for CandidateFinder<'_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let stmts: Vec<_> = node.body().iter().collect();
        self.check_statements(&stmts);
        ruby_prism::visit_statements_node(self, node);
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if let Some(body) = node.statements() {
            let stmts: Vec<_> = body.body().iter().collect();
            self.check_statements(&stmts);
        }
        ruby_prism::visit_begin_node(self, node);
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        self.check_tap_call(node);
        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapIntoArray::new(), "cops/style/map_into_array");
}
