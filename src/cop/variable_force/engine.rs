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
}

impl<'a> Engine<'a> {
    pub fn new(source: &'a SourceFile, consumers: &'a [RegisteredConsumer<'a>]) -> Self {
        Self {
            table: VariableTable::new(),
            source,
            consumers,
            diagnostics: Vec::new(),
        }
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
        self.visit(&node.value());
        self.table
            .assign_to_variable(&name, Assignment::new(offset, AssignmentKind::Simple));
    }

    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        let scope_index = self.table.current_scope_index();
        self.table.reference_variable(
            node.name().as_slice(),
            Reference::new(node.location().start_offset(), scope_index),
        );
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
        self.table
            .reference_variable(&name, Reference::new(offset, si));
        self.visit(&node.value());
        self.table
            .assign_to_variable(&name, Assignment::new(offset, AssignmentKind::Operator));
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
        self.table
            .reference_variable(&name, Reference::new(offset, si));
        self.visit(&node.value());
        self.table
            .assign_to_variable(&name, Assignment::new(offset, AssignmentKind::LogicalOr));
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
        self.table
            .reference_variable(&name, Reference::new(offset, si));
        self.visit(&node.value());
        self.table
            .assign_to_variable(&name, Assignment::new(offset, AssignmentKind::LogicalAnd));
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'pr>) {
        self.visit(&node.value());
        for target in node.lefts().iter() {
            if let Some(t) = target.as_local_variable_target_node() {
                let name = t.name().as_slice().to_vec();
                let offset = t.location().start_offset();
                if !self.table.variable_exists(&name) {
                    self.declare_variable(name.clone(), offset, DeclarationKind::Assignment);
                }
                self.table
                    .assign_to_variable(&name, Assignment::new(offset, AssignmentKind::Multiple));
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
                        self.table.assign_to_variable(
                            &name,
                            Assignment::new(offset, AssignmentKind::Rest),
                        );
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
                self.table
                    .assign_to_variable(&name, Assignment::new(offset, AssignmentKind::Multiple));
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
        self.enter_scope(kind, loc.start_offset(), loc.end_offset());
        if let Some(params) = node.parameters() {
            self.declare_parameters(&params);
        }
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.leave_scope();
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        let loc = node.location();
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
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        let loc = node.location();
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

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        self.visit(&node.predicate());
        if let Some(stmts) = node.statements() {
            for stmt in stmts.body().iter() {
                self.visit(&stmt);
            }
        }
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        self.visit(&node.predicate());
        if let Some(stmts) = node.statements() {
            for stmt in stmts.body().iter() {
                self.visit(&stmt);
            }
        }
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
            self.table
                .assign_to_variable(&name, Assignment::new(offset, AssignmentKind::For));
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
    use super::*;

    fn run_engine(source: &str) -> VariableTable {
        let sf = SourceFile::from_bytes("test.rb", source.as_bytes().to_vec());
        let pr = ruby_prism::parse(source.as_bytes());
        let consumers: Vec<RegisteredConsumer<'_>> = vec![];
        let mut engine = Engine::new(&sf, &consumers);
        engine.run(&pr);
        engine.table
    }

    #[test]
    fn test_simple_assignment_and_reference() {
        let _ = run_engine("x = 1\nputs x\n");
    }
    #[test]
    fn test_def_scope() {
        let _ = run_engine("x = 1\ndef foo\n  y = 2\nend\n");
    }
    #[test]
    fn test_block_scope() {
        let _ = run_engine("x = 1\n[1].each { |i| puts x }\n");
    }
    #[test]
    fn test_operator_assignment() {
        let _ = run_engine("x = 1\nx += 2\n");
    }
    #[test]
    fn test_class_scope() {
        let _ = run_engine("class Foo\n  x = 1\nend\n");
    }
    #[test]
    fn test_module_scope() {
        let _ = run_engine("module Foo\n  x = 1\nend\n");
    }
    #[test]
    fn test_for_loop() {
        let _ = run_engine("for x in [1, 2, 3]\n  puts x\nend\n");
    }
    #[test]
    fn test_multi_assignment() {
        let _ = run_engine("a, b = 1, 2\n");
    }
    #[test]
    fn test_lambda() {
        let _ = run_engine("f = -> (x) { x + 1 }\n");
    }
    #[test]
    fn test_singleton_class() {
        let _ = run_engine("obj = Object.new\nclass << obj\n  x = 1\nend\n");
    }
    #[test]
    fn test_binding_call() {
        let _ = run_engine("x = 1\nbinding\n");
    }
    #[test]
    fn test_forwarding_super() {
        let _ = run_engine("def foo(x)\n  super\nend\n");
    }
    #[test]
    fn test_nested_blocks() {
        let _ = run_engine("[1].each { |x| [2].each { |y| puts x + y } }\n");
    }
    #[test]
    fn test_all_param_types() {
        let _ = run_engine("def foo(a, b = 1, *c, d:, e: 2, **f, &g)\nend\n");
    }
    #[test]
    fn test_block_local_vars() {
        let _ = run_engine("[1].each { |x; local| local = x }\n");
    }
    #[test]
    fn test_or_and_write() {
        let _ = run_engine("x ||= 1\ny &&= 2\n");
    }
    #[test]
    fn test_class_with_superclass() {
        let _ = run_engine("base = Object\nclass Foo < base\nend\n");
    }
}
