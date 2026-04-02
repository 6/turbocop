//! The VariableForce AST visitor engine.
//!
//! Performs a single walk of the Prism AST, building a VariableTable and
//! dispatching hook callbacks to registered consumers at scope entry/exit
//! and variable declaration events.

use ruby_prism::Visit;

use super::VariableForceConsumer;
use super::assignment::{Assignment, AssignmentKind};
use super::reference::Reference;
use super::scope::ScopeKind;
use super::variable::DeclarationKind;
use super::variable_table::VariableTable;
use crate::cop::CopConfig;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// A registered consumer with its config.
pub struct RegisteredConsumer<'a> {
    pub consumer: &'a dyn VariableForceConsumer,
    pub config: &'a CopConfig,
}

/// The VariableForce engine. Walks the Prism AST and builds a complete
/// variable-scope model, dispatching hooks to consumers.
pub struct Engine<'a> {
    pub table: VariableTable,
    source: &'a SourceFile,
    consumers: &'a [RegisteredConsumer<'a>],
    diagnostics: Vec<Diagnostic>,
    /// Monotonically increasing counter for temporal ordering.
    sequence: usize,
    /// Depth inside conditional/branch constructs (if, unless, case, while,
    /// until, rescue, block, lambda). Assignments created while > 0 are
    /// marked `in_branch = true`.
    branch_depth: usize,
}

impl<'a> Engine<'a> {
    pub fn new(source: &'a SourceFile, consumers: &'a [RegisteredConsumer<'a>]) -> Self {
        Self {
            table: VariableTable::new(),
            source,
            consumers,
            diagnostics: Vec::new(),
            sequence: 0,
            branch_depth: 0,
        }
    }

    fn next_sequence(&mut self) -> usize {
        let seq = self.sequence;
        self.sequence += 1;
        seq
    }

    /// Run the engine on a parsed program node.
    pub fn run(&mut self, parse_result: &ruby_prism::ParseResult<'_>) {
        let root = parse_result.node();
        let program = match root.as_program_node() {
            Some(p) => p,
            None => return,
        };
        let loc = program.location();
        self.table
            .push_scope(ScopeKind::TopLevel, loc.start_offset(), loc.end_offset());
        self.fire_after_entering_scope();

        for stmt in program.statements().body().iter() {
            self.visit(&stmt);
        }

        self.fire_before_leaving_scope();
        self.table.pop_scope();
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    // ── Hook dispatch ──────────────────────────────────────────────────

    fn fire_after_entering_scope(&mut self) {
        let scope = self.table.current_scope();
        for rc in self.consumers {
            rc.consumer.after_entering_scope(
                scope,
                &self.table,
                self.source,
                rc.config,
                &mut self.diagnostics,
            );
        }
    }

    fn fire_before_leaving_scope(&mut self) {
        let scope = self.table.current_scope();
        for rc in self.consumers {
            rc.consumer.before_leaving_scope(
                scope,
                &self.table,
                self.source,
                rc.config,
                &mut self.diagnostics,
            );
        }
    }

    fn fire_after_leaving_scope(&mut self, scope: &super::Scope) {
        for rc in self.consumers {
            rc.consumer.after_leaving_scope(
                scope,
                &self.table,
                self.source,
                rc.config,
                &mut self.diagnostics,
            );
        }
    }

    // ── Scope management ───────────────────────────────────────────────

    fn enter_scope(&mut self, kind: ScopeKind, start: usize, end: usize) {
        self.table.push_scope(kind, start, end);
        self.fire_after_entering_scope();
    }

    fn leave_scope(&mut self) {
        self.fire_before_leaving_scope();
        let scope = self.table.pop_scope();
        self.fire_after_leaving_scope(&scope);
    }

    // ── Variable declaration with hooks ─────────────────────────────────

    fn declare_variable(&mut self, name: Vec<u8>, offset: usize, kind: DeclarationKind) {
        let temp_var =
            super::Variable::new(name.clone(), offset, kind, self.table.current_scope_index());
        for rc in self.consumers {
            rc.consumer.before_declaring_variable(
                &temp_var,
                &self.table,
                self.source,
                rc.config,
                &mut self.diagnostics,
            );
        }

        let created = self.table.declare_variable(name.clone(), offset, kind);
        if created {
            if let Some(var) = self.table.current_scope().variables.get(&name) {
                for rc in self.consumers {
                    rc.consumer.after_declaring_variable(
                        var,
                        &self.table,
                        self.source,
                        rc.config,
                        &mut self.diagnostics,
                    );
                }
            }
        }
    }

    // ── Parameter declaration ──────────────────────────────────────────

    fn declare_parameters(&mut self, params: &ruby_prism::ParametersNode<'_>) {
        for param in params.requireds().iter() {
            if let Some(rp) = param.as_required_parameter_node() {
                self.declare_variable(
                    rp.name().as_slice().to_vec(),
                    rp.location().start_offset(),
                    DeclarationKind::RequiredArg,
                );
            }
        }
        for param in params.optionals().iter() {
            if let Some(op) = param.as_optional_parameter_node() {
                self.declare_variable(
                    op.name().as_slice().to_vec(),
                    op.location().start_offset(),
                    DeclarationKind::OptionalArg,
                );
                self.visit(&op.value());
            }
        }
        if let Some(rest) = params.rest() {
            if let Some(rp) = rest.as_rest_parameter_node() {
                if let Some(name) = rp.name() {
                    self.declare_variable(
                        name.as_slice().to_vec(),
                        rp.location().start_offset(),
                        DeclarationKind::RestArg,
                    );
                }
            }
        }
        for param in params.posts().iter() {
            if let Some(rp) = param.as_required_parameter_node() {
                self.declare_variable(
                    rp.name().as_slice().to_vec(),
                    rp.location().start_offset(),
                    DeclarationKind::RequiredArg,
                );
            }
        }
        for param in params.keywords().iter() {
            if let Some(kp) = param.as_required_keyword_parameter_node() {
                let mut name = kp.name().as_slice().to_vec();
                if name.last() == Some(&b':') {
                    name.pop();
                }
                self.declare_variable(
                    name,
                    kp.location().start_offset(),
                    DeclarationKind::KeywordArg,
                );
            } else if let Some(kp) = param.as_optional_keyword_parameter_node() {
                let mut name = kp.name().as_slice().to_vec();
                if name.last() == Some(&b':') {
                    name.pop();
                }
                self.declare_variable(
                    name,
                    kp.location().start_offset(),
                    DeclarationKind::OptionalKeywordArg,
                );
                self.visit(&kp.value());
            }
        }
        if let Some(kw_rest) = params.keyword_rest() {
            if let Some(krp) = kw_rest.as_keyword_rest_parameter_node() {
                if let Some(name) = krp.name() {
                    self.declare_variable(
                        name.as_slice().to_vec(),
                        krp.location().start_offset(),
                        DeclarationKind::KeywordRestArg,
                    );
                }
            }
        }
        if let Some(block) = params.block() {
            if let Some(name) = block.name() {
                self.declare_variable(
                    name.as_slice().to_vec(),
                    block.location().start_offset(),
                    DeclarationKind::BlockArg,
                );
            }
        }
    }

    fn declare_block_parameters(&mut self, bp: &ruby_prism::BlockParametersNode<'_>) {
        if let Some(params) = bp.parameters() {
            self.declare_parameters(&params);
        }
        for local in bp.locals().iter() {
            if let Some(blv) = local.as_block_local_variable_node() {
                self.declare_variable(
                    blv.name().as_slice().to_vec(),
                    blv.location().start_offset(),
                    DeclarationKind::ShadowArg,
                );
            }
        }
    }
}

// ── Prism Visitor ──────────────────────────────────────────────────────

impl<'pr> Visit<'pr> for Engine<'_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let name = node.name().as_slice().to_vec();
        let offset = node.location().start_offset();
        if !self.table.variable_exists(&name) {
            self.declare_variable(name.clone(), offset, DeclarationKind::Assignment);
        }

        // Count explicit references before RHS to detect self-references.
        // Only explicit references count (not implicit ones from super/binding)
        // because RuboCop's `uses_var?` only matches `(lvar %)`.
        let explicit_refs_before = self
            .table
            .find_variable(&name)
            .map_or(0, |v| v.references.iter().filter(|r| r.explicit).count());

        self.visit(&node.value());

        let explicit_refs_after = self
            .table
            .find_variable(&name)
            .map_or(0, |v| v.references.iter().filter(|r| r.explicit).count());
        let rhs_refs_var = explicit_refs_after > explicit_refs_before;

        let seq = self.next_sequence();
        let mut assign = Assignment::new(offset, AssignmentKind::Simple);
        assign.sequence = seq;
        assign.rhs_references_var = rhs_refs_var;
        assign.in_branch = self.branch_depth > 0;
        self.table.assign_to_variable(&name, assign);
    }

    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        let scope_index = self.table.current_scope_index();
        let seq = self.next_sequence();
        let mut reference = Reference::new(node.location().start_offset(), scope_index);
        reference.sequence = seq;
        self.table
            .reference_variable(node.name().as_slice(), reference);
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        let name = node.name().as_slice().to_vec();
        let offset = node.location().start_offset();
        if !self.table.variable_exists(&name) {
            self.declare_variable(name.clone(), offset, DeclarationKind::Assignment);
        }
        let si = self.table.current_scope_index();
        let seq = self.next_sequence();
        let mut r = Reference::new(offset, si);
        r.sequence = seq;
        self.table.reference_variable(&name, r);
        self.visit(&node.value());
        let seq = self.next_sequence();
        let mut a = Assignment::new(offset, AssignmentKind::Operator);
        a.sequence = seq;
        a.rhs_references_var = true; // operator-writes always read the var
        a.in_branch = self.branch_depth > 0;
        self.table.assign_to_variable(&name, a);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        let name = node.name().as_slice().to_vec();
        let offset = node.location().start_offset();
        if !self.table.variable_exists(&name) {
            self.declare_variable(name.clone(), offset, DeclarationKind::Assignment);
        }
        let si = self.table.current_scope_index();
        let seq = self.next_sequence();
        let mut r = Reference::new(offset, si);
        r.sequence = seq;
        self.table.reference_variable(&name, r);
        self.visit(&node.value());
        let seq = self.next_sequence();
        let mut a = Assignment::new(offset, AssignmentKind::LogicalOr);
        a.sequence = seq;
        a.rhs_references_var = true;
        a.in_branch = self.branch_depth > 0;
        self.table.assign_to_variable(&name, a);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        let name = node.name().as_slice().to_vec();
        let offset = node.location().start_offset();
        if !self.table.variable_exists(&name) {
            self.declare_variable(name.clone(), offset, DeclarationKind::Assignment);
        }
        let si = self.table.current_scope_index();
        let seq = self.next_sequence();
        let mut r = Reference::new(offset, si);
        r.sequence = seq;
        self.table.reference_variable(&name, r);
        self.visit(&node.value());
        let seq = self.next_sequence();
        let mut a = Assignment::new(offset, AssignmentKind::LogicalAnd);
        a.sequence = seq;
        a.rhs_references_var = true;
        a.in_branch = self.branch_depth > 0;
        self.table.assign_to_variable(&name, a);
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        // Collect target names before visiting the RHS so we can detect self-refs
        let mut target_names: Vec<Vec<u8>> = Vec::new();
        for target in node.lefts().iter() {
            if let Some(t) = target.as_local_variable_target_node() {
                target_names.push(t.name().as_slice().to_vec());
            }
        }
        if let Some(rest) = node.rest() {
            if let Some(splat) = rest.as_splat_node() {
                if let Some(expr) = splat.expression() {
                    if let Some(t) = expr.as_local_variable_target_node() {
                        target_names.push(t.name().as_slice().to_vec());
                    }
                }
            }
        }
        for target in node.rights().iter() {
            if let Some(t) = target.as_local_variable_target_node() {
                target_names.push(t.name().as_slice().to_vec());
            }
        }

        // Bare `super` (ForwardingSuperNode) implicitly forwards all method
        // arguments, so any argument target in a `a, b = super` multi-write
        // is effectively "used" on the RHS.
        let rhs_is_forwarding_super = node.value().as_forwarding_super_node().is_some();

        // Snapshot explicit reference counts before RHS
        let refs_before: Vec<(Vec<u8>, usize)> = target_names
            .iter()
            .map(|name| {
                let count = self
                    .table
                    .find_variable(name)
                    .map_or(0, |v| v.references.iter().filter(|r| r.explicit).count());
                (name.clone(), count)
            })
            .collect();

        self.visit(&node.value());

        // Check which targets gained explicit references from the RHS.
        // If the RHS is bare `super`, treat all argument variables as referenced.
        let rhs_refs: Vec<(Vec<u8>, bool)> = refs_before
            .iter()
            .map(|(name, before)| {
                let explicitly_ref = {
                    let after = self
                        .table
                        .find_variable(name)
                        .map_or(0, |v| v.references.iter().filter(|r| r.explicit).count());
                    after > *before
                };
                let super_ref = rhs_is_forwarding_super
                    && self
                        .table
                        .find_variable(name)
                        .is_some_and(|v| v.is_argument());
                (name.clone(), explicitly_ref || super_ref)
            })
            .collect();

        let in_branch = self.branch_depth > 0;
        let seq = self.next_sequence();

        for target in node.lefts().iter() {
            if let Some(t) = target.as_local_variable_target_node() {
                let name = t.name().as_slice().to_vec();
                let offset = t.location().start_offset();
                if !self.table.variable_exists(&name) {
                    self.declare_variable(name.clone(), offset, DeclarationKind::Assignment);
                }
                let rhs_refs_var = rhs_refs
                    .iter()
                    .find(|(n, _)| n == &name)
                    .is_some_and(|(_, r)| *r);
                let mut a = Assignment::new(offset, AssignmentKind::Multiple);
                a.in_branch = in_branch;
                a.sequence = seq;
                a.rhs_references_var = rhs_refs_var;
                self.table.assign_to_variable(&name, a);
            } else {
                self.visit(&target);
            }
        }
        if let Some(rest) = node.rest() {
            if let Some(splat) = rest.as_splat_node() {
                if let Some(expr) = splat.expression() {
                    if let Some(t) = expr.as_local_variable_target_node() {
                        let name = t.name().as_slice().to_vec();
                        let offset = t.location().start_offset();
                        if !self.table.variable_exists(&name) {
                            self.declare_variable(
                                name.clone(),
                                offset,
                                DeclarationKind::Assignment,
                            );
                        }
                        let rhs_refs_var = rhs_refs
                            .iter()
                            .find(|(n, _)| n == &name)
                            .is_some_and(|(_, r)| *r);
                        let mut a = Assignment::new(offset, AssignmentKind::Rest);
                        a.in_branch = in_branch;
                        a.sequence = seq;
                        a.rhs_references_var = rhs_refs_var;
                        self.table.assign_to_variable(&name, a);
                    }
                }
            } else {
                self.visit(&rest);
            }
        }
        for target in node.rights().iter() {
            if let Some(t) = target.as_local_variable_target_node() {
                let name = t.name().as_slice().to_vec();
                let offset = t.location().start_offset();
                if !self.table.variable_exists(&name) {
                    self.declare_variable(name.clone(), offset, DeclarationKind::Assignment);
                }
                let rhs_refs_var = rhs_refs
                    .iter()
                    .find(|(n, _)| n == &name)
                    .is_some_and(|(_, r)| *r);
                let mut a = Assignment::new(offset, AssignmentKind::Multiple);
                a.in_branch = in_branch;
                a.sequence = seq;
                a.rhs_references_var = rhs_refs_var;
                self.table.assign_to_variable(&name, a);
            } else {
                self.visit(&target);
            }
        }
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        let kind = if node.receiver().is_some() {
            ScopeKind::Defs
        } else {
            ScopeKind::Def
        };
        let loc = node.location();
        let saved_depth = self.branch_depth;
        self.branch_depth = 0;
        self.enter_scope(kind, loc.start_offset(), loc.end_offset());
        if let Some(params) = node.parameters() {
            self.declare_parameters(&params);
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.leave_scope();
        self.branch_depth = saved_depth;
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let loc = node.location();
        // Save and reset branch_depth: block body starts a fresh scope.
        // Assignments to outer variables are marked captured_by_block by the
        // variable table, which the cop uses as a conditional indicator.
        let saved_depth = self.branch_depth;
        self.branch_depth = 0;
        self.enter_scope(ScopeKind::Block, loc.start_offset(), loc.end_offset());
        if let Some(params) = node.parameters() {
            if let Some(bp) = params.as_block_parameters_node() {
                self.declare_block_parameters(&bp);
            }
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.leave_scope();
        self.branch_depth = saved_depth;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let loc = node.location();
        let saved_depth = self.branch_depth;
        self.branch_depth = 0;
        self.enter_scope(ScopeKind::Block, loc.start_offset(), loc.end_offset());
        if let Some(params) = node.parameters() {
            if let Some(bp) = params.as_block_parameters_node() {
                self.declare_block_parameters(&bp);
            }
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.leave_scope();
        self.branch_depth = saved_depth;
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(superclass) = node.superclass() {
            self.visit(&superclass);
        }
        let loc = node.location();
        self.enter_scope(ScopeKind::Class, loc.start_offset(), loc.end_offset());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.leave_scope();
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let loc = node.location();
        self.enter_scope(ScopeKind::Module, loc.start_offset(), loc.end_offset());
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.leave_scope();
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        self.visit(&node.expression());
        let loc = node.location();
        self.enter_scope(
            ScopeKind::SingletonClass,
            loc.start_offset(),
            loc.end_offset(),
        );
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.leave_scope();
    }

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.visit(&node.predicate());
        self.branch_depth += 1;
        if let Some(stmts) = node.statements() {
            for stmt in stmts.body().iter() {
                self.visit(&stmt);
            }
        }
        self.branch_depth -= 1;
        if let Some(subsequent) = node.subsequent() {
            self.branch_depth += 1;
            self.visit(&subsequent);
            self.branch_depth -= 1;
        }
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.visit(&node.predicate());
        self.branch_depth += 1;
        if let Some(stmts) = node.statements() {
            for stmt in stmts.body().iter() {
                self.visit(&stmt);
            }
        }
        if let Some(else_clause) = node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for stmt in stmts.body().iter() {
                    self.visit(&stmt);
                }
            }
        }
        self.branch_depth -= 1;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        if let Some(pred) = node.predicate() {
            self.visit(&pred);
        }
        self.branch_depth += 1;
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        if let Some(else_clause) = node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for stmt in stmts.body().iter() {
                    self.visit(&stmt);
                }
            }
        }
        self.branch_depth -= 1;
    }

    fn visit_case_match_node(&mut self, node: &ruby_prism::CaseMatchNode<'pr>) {
        if let Some(pred) = node.predicate() {
            self.visit(&pred);
        }
        self.branch_depth += 1;
        for condition in node.conditions().iter() {
            self.visit(&condition);
        }
        if let Some(else_clause) = node.else_clause() {
            if let Some(stmts) = else_clause.statements() {
                for stmt in stmts.body().iter() {
                    self.visit(&stmt);
                }
            }
        }
        self.branch_depth -= 1;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        self.visit(&node.predicate());
        self.branch_depth += 1;
        if let Some(stmts) = node.statements() {
            for stmt in stmts.body().iter() {
                self.visit(&stmt);
            }
        }
        self.branch_depth -= 1;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        self.visit(&node.predicate());
        self.branch_depth += 1;
        if let Some(stmts) = node.statements() {
            for stmt in stmts.body().iter() {
                self.visit(&stmt);
            }
        }
        self.branch_depth -= 1;
    }

    fn visit_rescue_node(&mut self, node: &ruby_prism::RescueNode<'pr>) {
        self.branch_depth += 1;
        ruby_prism::visit_rescue_node(self, node);
        self.branch_depth -= 1;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        self.branch_depth += 1;
        ruby_prism::visit_begin_node(self, node);
        self.branch_depth -= 1;
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        self.visit(&node.collection());
        let index = node.index();
        if let Some(target) = index.as_local_variable_target_node() {
            let name = target.name().as_slice().to_vec();
            let offset = target.location().start_offset();
            if !self.table.variable_exists(&name) {
                self.declare_variable(name.clone(), offset, DeclarationKind::ForIndex);
            }
            let mut a = Assignment::new(offset, AssignmentKind::For);
            a.in_branch = self.branch_depth > 0;
            self.table.assign_to_variable(&name, a);
        } else {
            self.visit(&index);
        }
        if let Some(stmts) = node.statements() {
            for stmt in stmts.body().iter() {
                self.visit(&stmt);
            }
        }
    }

    fn visit_forwarding_super_node(&mut self, node: &ruby_prism::ForwardingSuperNode<'pr>) {
        let offset = node.location().start_offset();
        let si = self.table.current_scope_index();
        for var in self.table.accessible_variables_mut() {
            if var.is_argument() {
                var.reference(Reference::implicit(offset, si));
            }
        }
    }

    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"binding" && node.arguments().is_none() {
            let offset = node.location().start_offset();
            let si = self.table.current_scope_index();
            for var in self.table.accessible_variables_mut() {
                var.reference(Reference::implicit(offset, si));
            }
        }
        if let Some(recv) = node.receiver() {
            self.visit(&recv);
        }
        if let Some(args) = node.arguments() {
            for arg in args.arguments().iter() {
                self.visit(&arg);
            }
        }
        if let Some(block) = node.block() {
            self.visit(&block);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::*;
    use crate::cop::variable_force::variable::DeclarationKind;

    /// A test consumer that collects scope/variable data during hooks.
    struct TestConsumer {
        /// Variables seen in before_leaving_scope, keyed by scope kind.
        /// Each entry: (scope_kind, {var_name: (assignments_count, references_count, declaration_kind)})
        scopes: RefCell<Vec<ScopeSnapshot>>,
        /// Variables seen in before_declaring_variable (for shadowing tests).
        declarations: RefCell<Vec<(Vec<u8>, bool)>>, // (name, outer_exists)
    }

    #[derive(Debug)]
    struct ScopeSnapshot {
        kind: ScopeKind,
        vars: HashMap<String, VarSnapshot>,
    }

    #[derive(Debug)]
    struct VarSnapshot {
        decl_kind: DeclarationKind,
        num_assignments: usize,
        num_references: usize,
        captured_by_block: bool,
        used: bool,
        has_implicit_ref: bool,
        /// Whether any assignment has rhs_references_var set.
        has_self_ref_assignment: bool,
    }

    impl TestConsumer {
        fn new() -> Self {
            Self {
                scopes: RefCell::new(Vec::new()),
                declarations: RefCell::new(Vec::new()),
            }
        }
    }

    impl VariableForceConsumer for TestConsumer {
        fn before_leaving_scope(
            &self,
            scope: &super::super::Scope,
            _table: &VariableTable,
            _source: &SourceFile,
            _config: &CopConfig,
            _diagnostics: &mut Vec<Diagnostic>,
        ) {
            let mut vars = HashMap::new();
            for (name, var) in &scope.variables {
                vars.insert(
                    String::from_utf8_lossy(name).to_string(),
                    VarSnapshot {
                        decl_kind: var.declaration_kind,
                        num_assignments: var.assignments.len(),
                        num_references: var.references.len(),
                        captured_by_block: var.captured_by_block,
                        used: var.used(),
                        has_implicit_ref: var.references.iter().any(|r| !r.explicit),
                        has_self_ref_assignment: var
                            .assignments
                            .iter()
                            .any(|a| a.rhs_references_var),
                    },
                );
            }
            self.scopes.borrow_mut().push(ScopeSnapshot {
                kind: scope.kind,
                vars,
            });
        }

        fn before_declaring_variable(
            &self,
            variable: &super::super::Variable,
            table: &VariableTable,
            _source: &SourceFile,
            _config: &CopConfig,
            _diagnostics: &mut Vec<Diagnostic>,
        ) {
            let outer_exists = table.find_variable(&variable.name).is_some();
            self.declarations
                .borrow_mut()
                .push((variable.name.clone(), outer_exists));
        }
    }

    // We need Send+Sync for the trait bounds
    unsafe impl Send for TestConsumer {}
    unsafe impl Sync for TestConsumer {}

    fn run_with_consumer(source: &str) -> (Vec<ScopeSnapshot>, Vec<(Vec<u8>, bool)>) {
        let sf = SourceFile::from_bytes("test.rb", source.as_bytes().to_vec());
        let pr = ruby_prism::parse(source.as_bytes());
        let consumer = TestConsumer::new();
        let config = CopConfig::default();
        let rc = vec![RegisteredConsumer {
            consumer: &consumer,
            config: &config,
        }];
        let mut engine = Engine::new(&sf, &rc);
        engine.run(&pr);
        let scopes = consumer.scopes.into_inner();
        let decls = consumer.declarations.into_inner();
        (scopes, decls)
    }

    fn run_engine(source: &str) -> Vec<ScopeSnapshot> {
        run_with_consumer(source).0
    }

    // ── Variable tracking tests ────────────────────────────────────────

    #[test]
    fn test_assignment_and_reference_tracked() {
        let scopes = run_engine("x = 1\nputs x\n");
        // TopLevel scope should have variable x
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0].kind, ScopeKind::TopLevel);
        let x = &scopes[0].vars["x"];
        assert_eq!(x.num_assignments, 1);
        assert_eq!(x.num_references, 1);
        assert!(x.used);
    }

    #[test]
    fn test_unused_variable() {
        let scopes = run_engine("x = 1\n");
        let x = &scopes[0].vars["x"];
        assert_eq!(x.num_assignments, 1);
        assert_eq!(x.num_references, 0);
        assert!(!x.used);
    }

    #[test]
    fn test_multiple_assignments() {
        let scopes = run_engine("x = 1\nx = 2\nputs x\n");
        let x = &scopes[0].vars["x"];
        assert_eq!(x.num_assignments, 2);
        assert_eq!(x.num_references, 1);
    }

    #[test]
    fn test_self_referencing_assignment() {
        // x = x + 1 should create a reference BEFORE the second assignment
        let scopes = run_engine("x = 1\nx = x + 1\n");
        let x = &scopes[0].vars["x"];
        assert_eq!(x.num_assignments, 2);
        assert_eq!(x.num_references, 1); // x on RHS of second assignment
        assert!(x.has_self_ref_assignment); // second assignment references x on RHS
    }

    #[test]
    fn test_non_self_referencing_assignment() {
        let scopes = run_engine("x = 1\nx = 2\n");
        let x = &scopes[0].vars["x"];
        assert!(!x.has_self_ref_assignment); // x = 2 does NOT reference x
    }

    #[test]
    fn test_operator_write_always_self_refs() {
        let scopes = run_engine("x = 1\nx += 2\n");
        let x = &scopes[0].vars["x"];
        assert!(x.has_self_ref_assignment); // += always reads x
    }

    #[test]
    fn test_operator_assignment_creates_reference() {
        let scopes = run_engine("x = 1\nx += 2\n");
        let x = &scopes[0].vars["x"];
        assert_eq!(x.num_assignments, 2); // x = 1, x += 2
        assert_eq!(x.num_references, 1); // += reads x
        assert!(x.used);
    }

    #[test]
    fn test_or_write_creates_reference() {
        let scopes = run_engine("x = nil\nx ||= 1\n");
        let x = &scopes[0].vars["x"];
        assert_eq!(x.num_assignments, 2);
        assert_eq!(x.num_references, 1); // ||= reads x
    }

    #[test]
    fn test_and_write_creates_reference() {
        let scopes = run_engine("x = true\nx &&= false\n");
        let x = &scopes[0].vars["x"];
        assert_eq!(x.num_assignments, 2);
        assert_eq!(x.num_references, 1);
    }

    // ── Scope boundary tests ───────────────────────────────────────────

    #[test]
    fn test_def_is_hard_scope() {
        let scopes = run_engine("x = 1\ndef foo\n  y = 2\n  puts x\nend\n");
        // Should have 2 scopes: TopLevel and Def
        assert_eq!(scopes.len(), 2);

        // Def scope has y but NOT x (hard boundary)
        let def_scope = &scopes[0]; // inner scope popped first
        assert_eq!(def_scope.kind, ScopeKind::Def);
        assert!(def_scope.vars.contains_key("y"));
        assert!(!def_scope.vars.contains_key("x"));

        // TopLevel has x
        let top_scope = &scopes[1];
        assert_eq!(top_scope.kind, ScopeKind::TopLevel);
        assert!(top_scope.vars.contains_key("x"));
        // x is NOT referenced (the `puts x` inside def can't see it)
        assert_eq!(top_scope.vars["x"].num_references, 0);
    }

    #[test]
    fn test_block_captures_outer_variable() {
        let scopes = run_engine("x = 1\n[1].each { |i| puts x }\n");
        // Block scope and TopLevel scope
        assert_eq!(scopes.len(), 2);

        let block_scope = &scopes[0];
        assert_eq!(block_scope.kind, ScopeKind::Block);
        assert!(block_scope.vars.contains_key("i"));

        let top_scope = &scopes[1];
        assert!(top_scope.vars.contains_key("x"));
        // x IS referenced (block captures it) and captured_by_block
        assert_eq!(top_scope.vars["x"].num_references, 1);
        assert!(top_scope.vars["x"].captured_by_block);
    }

    #[test]
    fn test_class_is_hard_scope() {
        let scopes = run_engine("x = 1\nclass Foo\n  y = 2\nend\n");
        let class_scope = &scopes[0];
        assert_eq!(class_scope.kind, ScopeKind::Class);
        assert!(class_scope.vars.contains_key("y"));
        assert!(!class_scope.vars.contains_key("x"));
    }

    #[test]
    fn test_module_is_hard_scope() {
        let scopes = run_engine("x = 1\nmodule Foo\n  y = 2\nend\n");
        let mod_scope = &scopes[0];
        assert_eq!(mod_scope.kind, ScopeKind::Module);
        assert!(mod_scope.vars.contains_key("y"));
    }

    #[test]
    fn test_class_superclass_in_outer_scope() {
        let scopes = run_engine("base = Object\nclass Foo < base\n  x = 1\nend\n");
        // `base` should be referenced in the TopLevel scope (outer), not the Class scope
        let top = scopes
            .iter()
            .find(|s| s.kind == ScopeKind::TopLevel)
            .unwrap();
        assert!(top.vars["base"].num_references > 0);
    }

    #[test]
    fn test_singleton_class_receiver_in_outer_scope() {
        let scopes = run_engine("obj = Object.new\nclass << obj\n  x = 1\nend\n");
        let top = scopes
            .iter()
            .find(|s| s.kind == ScopeKind::TopLevel)
            .unwrap();
        assert!(top.vars["obj"].num_references > 0);
    }

    // ── Parameter declaration tests ────────────────────────────────────

    #[test]
    fn test_method_params_declared() {
        let scopes = run_engine("def foo(a, b = 1, *c, d:, e: 2, **f, &g)\nend\n");
        let def_scope = &scopes[0];
        assert_eq!(def_scope.kind, ScopeKind::Def);
        for name in &["a", "b", "c", "d", "e", "f", "g"] {
            assert!(def_scope.vars.contains_key(*name), "missing param: {name}");
        }
        assert_eq!(def_scope.vars["a"].decl_kind, DeclarationKind::RequiredArg);
        assert_eq!(def_scope.vars["b"].decl_kind, DeclarationKind::OptionalArg);
        assert_eq!(def_scope.vars["c"].decl_kind, DeclarationKind::RestArg);
        assert_eq!(def_scope.vars["d"].decl_kind, DeclarationKind::KeywordArg);
        assert_eq!(
            def_scope.vars["e"].decl_kind,
            DeclarationKind::OptionalKeywordArg
        );
        assert_eq!(
            def_scope.vars["f"].decl_kind,
            DeclarationKind::KeywordRestArg
        );
        assert_eq!(def_scope.vars["g"].decl_kind, DeclarationKind::BlockArg);
    }

    #[test]
    fn test_block_params_declared() {
        let scopes = run_engine("[1].each { |x, *y; local| }\n");
        let block_scope = &scopes[0];
        assert!(block_scope.vars.contains_key("x"));
        assert!(block_scope.vars.contains_key("y"));
        assert!(block_scope.vars.contains_key("local"));
        assert_eq!(
            block_scope.vars["local"].decl_kind,
            DeclarationKind::ShadowArg
        );
    }

    #[test]
    fn test_lambda_params_declared() {
        let scopes = run_engine("f = -> (x, y) { x + y }\n");
        let lambda_scope = &scopes[0];
        assert_eq!(lambda_scope.kind, ScopeKind::Block);
        assert!(lambda_scope.vars.contains_key("x"));
        assert!(lambda_scope.vars.contains_key("y"));
    }

    // ── Special node tests ─────────────────────────────────────────────

    #[test]
    fn test_binding_references_all_vars() {
        let scopes = run_engine("def foo(x)\n  y = 1\n  binding\nend\n");
        let def_scope = &scopes[0];
        // binding should reference both x and y
        assert!(def_scope.vars["x"].num_references > 0);
        assert!(def_scope.vars["y"].num_references > 0);
        // references from binding are implicit
        assert!(def_scope.vars["x"].has_implicit_ref);
    }

    #[test]
    fn test_forwarding_super_references_args() {
        let scopes = run_engine("def foo(x, y)\n  super\nend\n");
        let def_scope = &scopes[0];
        assert!(def_scope.vars["x"].num_references > 0);
        assert!(def_scope.vars["y"].num_references > 0);
        assert!(def_scope.vars["x"].has_implicit_ref);
    }

    #[test]
    fn test_forwarding_super_does_not_ref_locals() {
        let scopes = run_engine("def foo(x)\n  y = 1\n  super\nend\n");
        let def_scope = &scopes[0];
        assert!(def_scope.vars["x"].num_references > 0); // arg referenced
        assert_eq!(def_scope.vars["y"].num_references, 0); // local NOT referenced
    }

    // ── Multi-write tests ──────────────────────────────────────────────

    #[test]
    fn test_multi_write() {
        let scopes = run_engine("a, b = 1, 2\nputs a\n");
        let top = &scopes[0];
        assert!(top.vars.contains_key("a"));
        assert!(top.vars.contains_key("b"));
        assert_eq!(top.vars["a"].num_assignments, 1);
        assert_eq!(top.vars["b"].num_assignments, 1);
        assert!(top.vars["a"].used);
        assert!(!top.vars["b"].used);
    }

    #[test]
    fn test_multi_write_with_splat() {
        let scopes = run_engine("a, *b = [1, 2, 3]\n");
        let top = &scopes[0];
        assert!(top.vars.contains_key("a"));
        assert!(top.vars.contains_key("b"));
    }

    // ── For loop tests ─────────────────────────────────────────────────

    #[test]
    fn test_for_loop_index_variable() {
        let scopes = run_engine("for x in [1, 2, 3]\n  puts x\nend\n");
        // for loop doesn't create a new scope — x is in TopLevel
        let top = &scopes[0];
        assert!(top.vars.contains_key("x"));
        assert_eq!(top.vars["x"].decl_kind, DeclarationKind::ForIndex);
        assert!(top.vars["x"].used);
    }

    // ── Nested scope tests ─────────────────────────────────────────────

    #[test]
    fn test_nested_blocks_capture_outer() {
        let scopes = run_engine("x = 1\n[1].each { |i| [2].each { |j| puts x } }\n");
        let top = scopes
            .iter()
            .find(|s| s.kind == ScopeKind::TopLevel)
            .unwrap();
        assert!(top.vars["x"].captured_by_block);
        assert!(top.vars["x"].used);
    }

    #[test]
    fn test_def_inside_block() {
        // def creates a hard boundary even inside a block
        let scopes = run_engine("x = 1\n[1].each { |i| def bar; y = x; end }\n");
        let top = scopes
            .iter()
            .find(|s| s.kind == ScopeKind::TopLevel)
            .unwrap();
        // x should NOT be referenced (def is a hard boundary, can't see x)
        assert_eq!(top.vars["x"].num_references, 0);
    }

    // ── before_declaring_variable hook tests ───────────────────────────

    #[test]
    fn test_block_param_shadows_outer() {
        let (_, decls) = run_with_consumer("x = 1\n[1].each { |x| puts x }\n");
        // The second declaration of 'x' (block param) should see outer_exists = true
        let x_decls: Vec<_> = decls.iter().filter(|(n, _)| n == b"x").collect();
        assert_eq!(x_decls.len(), 2);
        assert!(!x_decls[0].1); // first x = 1, no outer
        assert!(x_decls[1].1); // block param x, outer exists
    }

    #[test]
    fn test_no_shadow_in_def() {
        let (_, decls) = run_with_consumer("x = 1\ndef foo(x)\nend\n");
        let x_decls: Vec<_> = decls.iter().filter(|(n, _)| n == b"x").collect();
        assert_eq!(x_decls.len(), 2);
        assert!(!x_decls[0].1); // first x = 1
        assert!(!x_decls[1].1); // def param x — hard scope, no outer visible
    }

    // ── Defs (singleton method) tests ──────────────────────────────────

    #[test]
    fn test_defs_receiver_in_outer_scope() {
        let scopes = run_engine("obj = Object.new\ndef obj.foo\n  x = 1\nend\n");
        let top = scopes
            .iter()
            .find(|s| s.kind == ScopeKind::TopLevel)
            .unwrap();
        assert!(top.vars["obj"].num_references > 0);
    }

    #[test]
    fn test_defs_scope_kind() {
        let scopes = run_engine("def self.foo(x)\nend\n");
        let defs = scopes.iter().find(|s| s.kind == ScopeKind::Defs).unwrap();
        assert!(defs.vars.contains_key("x"));
    }
}
