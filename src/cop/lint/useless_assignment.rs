use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use ruby_prism::Visit;

use crate::cop::variable_force::engine::{Engine, RegisteredConsumer};
use crate::cop::variable_force::{self, Scope, VariableTable};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for every useless assignment to local variable in every scope.
///
/// ## Implementation
///
/// This cop runs VariableForce inside `check_source`, then post-processes the
/// pending offenses before emitting diagnostics. The shared engine still does
/// the variable lifetime analysis; this wrapper only filters known
/// RuboCop-compatible false positives.
///
/// ## FP fix: chained `rescue` clauses (2026-04-03)
///
/// VariableForce currently models all clauses in a `begin ... rescue ...
/// rescue ... end` chain as one branch. That makes assignments in earlier
/// rescue clauses look overwritten by later rescue-clause assignments even
/// though the clauses are mutually exclusive. RuboCop keeps those writes live
/// until a real overwrite or read outside the sibling rescue set.
///
/// Fixed here by suppressing only the pending offenses whose later "overwrite"
/// comes exclusively from sibling rescue clauses in the same rescue chain, and
/// whose value is later read before any real overwrite on the same path.
pub struct UselessAssignment;

impl Cop for UselessAssignment {
    fn name(&self) -> &'static str {
        "Lint/UselessAssignment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let collector = PendingOffenseCollector::default();
        let consumers = [RegisteredConsumer {
            consumer: &collector,
            config,
        }];
        let mut engine = Engine::new(source, &consumers);
        engine.run(parse_result);
        let _ = engine.into_diagnostics();

        let rescue_contexts = collect_multi_rescue_contexts(parse_result);
        let conditional_operator_offsets = collect_conditional_operator_write_offsets(parse_result);
        let mut candidates = collector.take_candidates();
        candidates.sort_by_key(|candidate| candidate.node_offset);

        for candidate in candidates {
            let emit = if conditional_operator_offsets.contains(&candidate.node_offset) {
                true
            } else if !candidate.engine_used {
                !should_suppress_multi_rescue_false_positive(&candidate, &rescue_contexts)
            } else {
                false
            };

            if !emit {
                continue;
            }

            let (line, column) = source.offset_to_line_col(candidate.node_offset);
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!(
                    "Useless assignment to variable - `{}`.",
                    String::from_utf8_lossy(&candidate.name)
                ),
            ));
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct AssignmentState {
    offset: usize,
    branch_id: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct ReferenceState {
    offset: usize,
    branch_id: Option<usize>,
}

#[derive(Debug, Clone)]
struct AssignmentCandidate {
    name: Vec<u8>,
    node_offset: usize,
    branch_id: Option<usize>,
    engine_used: bool,
    assignment_states: Vec<AssignmentState>,
    reference_states: Vec<ReferenceState>,
}

#[derive(Default)]
struct PendingOffenseCollector {
    candidates: Mutex<Vec<AssignmentCandidate>>,
}

impl PendingOffenseCollector {
    fn take_candidates(&self) -> Vec<AssignmentCandidate> {
        std::mem::take(&mut *self.candidates.lock().unwrap())
    }
}

impl variable_force::VariableForceConsumer for PendingOffenseCollector {
    fn before_leaving_scope(
        &self,
        scope: &Scope,
        _variable_table: &VariableTable,
        _source: &SourceFile,
        _config: &CopConfig,
        _diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut candidates = self.candidates.lock().unwrap();

        for variable in scope.variables.values() {
            if variable.should_be_unused() {
                continue;
            }

            let assignment_states: Vec<_> = variable
                .assignments
                .iter()
                .map(|assignment| AssignmentState {
                    offset: assignment.node_offset,
                    branch_id: assignment.branch_id,
                })
                .collect();
            let reference_states: Vec<_> = variable
                .references
                .iter()
                .map(|reference| ReferenceState {
                    offset: reference.node_offset,
                    branch_id: reference.branch_id,
                })
                .collect();
            for assignment in &variable.assignments {
                candidates.push(AssignmentCandidate {
                    name: variable.name.clone(),
                    node_offset: assignment.node_offset,
                    branch_id: assignment.branch_id,
                    engine_used: assignment.used(variable.captured_by_block),
                    assignment_states: assignment_states.clone(),
                    reference_states: reference_states.clone(),
                });
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RescueClauseContext {
    begin_offset: usize,
    clause_index: usize,
}

#[derive(Default)]
struct MultiRescueContexts {
    assignments: HashMap<usize, RescueClauseContext>,
    references: HashMap<usize, RescueClauseContext>,
}

fn collect_multi_rescue_contexts(
    parse_result: &ruby_prism::ParseResult<'_>,
) -> MultiRescueContexts {
    let mut collector = MultiRescueCollector::default();
    collector.visit(&parse_result.node());
    collector.contexts
}

#[derive(Default)]
struct MultiRescueCollector {
    contexts: MultiRescueContexts,
    rescue_stack: Vec<RescueClauseContext>,
}

impl MultiRescueCollector {
    fn visit_rescue_clause_body(
        &mut self,
        rescue: ruby_prism::RescueNode<'_>,
        begin_offset: usize,
        clause_index: usize,
        multi_clause: bool,
    ) {
        if multi_clause {
            self.rescue_stack.push(RescueClauseContext {
                begin_offset,
                clause_index,
            });
        }

        for exception in rescue.exceptions().iter() {
            self.visit(&exception);
        }
        if let Some(reference) = rescue.reference() {
            self.visit(&reference);
        }
        if let Some(statements) = rescue.statements() {
            for statement in statements.body().iter() {
                self.visit(&statement);
            }
        }

        if multi_clause {
            self.rescue_stack.pop();
        }
    }
}

impl<'pr> Visit<'pr> for MultiRescueCollector {
    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if let Some(statements) = node.statements() {
            for statement in statements.body().iter() {
                self.visit(&statement);
            }
        }

        if let Some(first_rescue) = node.rescue_clause() {
            let begin_offset = node.location().start_offset();
            let mut clauses = Vec::new();
            let mut current = Some(first_rescue);
            while let Some(rescue) = current {
                let next = rescue.subsequent();
                clauses.push(rescue);
                current = next;
            }

            let multi_clause = clauses.len() > 1;
            for (clause_index, rescue) in clauses.into_iter().enumerate() {
                self.visit_rescue_clause_body(rescue, begin_offset, clause_index, multi_clause);
            }
        }

        if let Some(else_clause) = node.else_clause() {
            if let Some(statements) = else_clause.statements() {
                for statement in statements.body().iter() {
                    self.visit(&statement);
                }
            }
        }

        if let Some(ensure_clause) = node.ensure_clause() {
            if let Some(statements) = ensure_clause.statements() {
                for statement in statements.body().iter() {
                    self.visit(&statement);
                }
            }
        }
    }

    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if let Some(context) = self.rescue_stack.last().copied() {
            self.contexts
                .assignments
                .insert(node.location().start_offset(), context);
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if let Some(context) = self.rescue_stack.last().copied() {
            self.contexts
                .references
                .insert(node.location().start_offset(), context);
        }
    }
}

fn should_suppress_multi_rescue_false_positive(
    offense: &AssignmentCandidate,
    contexts: &MultiRescueContexts,
) -> bool {
    let Some(context) = contexts.assignments.get(&offense.node_offset).copied() else {
        return false;
    };

    let next_real_assignment = offense
        .assignment_states
        .iter()
        .copied()
        .filter(|assignment| assignment.offset > offense.node_offset)
        .filter(|assignment| {
            !is_sibling_multi_rescue_assignment(context, assignment.offset, contexts)
                && (assignment.branch_id.is_none() || assignment.branch_id == offense.branch_id)
        })
        .map(|assignment| assignment.offset)
        .min()
        .unwrap_or(usize::MAX);

    offense
        .reference_states
        .iter()
        .filter(|reference| {
            reference.offset > offense.node_offset && reference.offset < next_real_assignment
        })
        .any(|reference| !is_sibling_multi_rescue_reference(context, reference.offset, contexts))
}

fn is_sibling_multi_rescue_assignment(
    current: RescueClauseContext,
    offset: usize,
    contexts: &MultiRescueContexts,
) -> bool {
    contexts.assignments.get(&offset).is_some_and(|other| {
        other.begin_offset == current.begin_offset && other.clause_index != current.clause_index
    })
}

fn is_sibling_multi_rescue_reference(
    current: RescueClauseContext,
    offset: usize,
    contexts: &MultiRescueContexts,
) -> bool {
    contexts.references.get(&offset).is_some_and(|other| {
        other.begin_offset == current.begin_offset && other.clause_index != current.clause_index
    })
}

fn collect_conditional_operator_write_offsets(
    parse_result: &ruby_prism::ParseResult<'_>,
) -> HashSet<usize> {
    let mut collector = ConditionalOperatorWriteCollector::default();
    collector.visit(&parse_result.node());
    collector.offsets
}

#[derive(Default)]
struct ConditionalOperatorWriteCollector {
    offsets: HashSet<usize>,
}

impl<'pr> Visit<'pr> for ConditionalOperatorWriteCollector {
    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        if let (Some(if_body), Some(subsequent)) = (node.statements(), node.subsequent()) {
            if let Some(else_node) = subsequent.as_else_node() {
                let if_stmt = single_statement_from_statements(&if_body);
                let else_stmt = else_node
                    .statements()
                    .and_then(|statements| single_statement_from_statements(&statements));

                if let (Some(if_stmt), Some(else_stmt)) = (if_stmt, else_stmt) {
                    if let Some(offset) = matching_operator_write_offset(&if_stmt, &else_stmt) {
                        self.offsets.insert(offset);
                    }
                    if let Some(offset) = matching_operator_write_offset(&else_stmt, &if_stmt) {
                        self.offsets.insert(offset);
                    }
                }
            }
        }

        ruby_prism::visit_if_node(self, node);
    }
}

fn single_statement_from_statements<'pr>(
    statements: &ruby_prism::StatementsNode<'pr>,
) -> Option<ruby_prism::Node<'pr>> {
    let mut body = statements.body().iter();
    let statement = body.next()?;
    if body.next().is_some() {
        return None;
    }
    Some(statement)
}

fn matching_operator_write_offset(
    write_branch: &ruby_prism::Node<'_>,
    read_branch: &ruby_prism::Node<'_>,
) -> Option<usize> {
    let write = write_branch.as_local_variable_operator_write_node()?;
    let read = read_branch.as_local_variable_read_node()?;
    if write.name().as_slice() == read.name().as_slice() {
        Some(write.location().start_offset())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UselessAssignment, "cops/lint/useless_assignment");
}
