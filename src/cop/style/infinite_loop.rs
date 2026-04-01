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
pub struct InfiniteLoop;

impl Cop for InfiniteLoop {
    fn name(&self) -> &'static str {
        "Style/InfiniteLoop"
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
        let mut visitor = InfiniteLoopVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            scope_stack: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct InfiniteLoopVisitor<'a, 'pr> {
    cop: &'a InfiniteLoop,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    scope_stack: Vec<ruby_prism::Node<'pr>>,
}

/// Returns true if the node is a truthy literal (true, integer, float, array, hash).
fn is_truthy_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_true_node().is_some()
        || node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_array_node().is_some()
        || node.as_hash_node().is_some()
        || node.as_keyword_hash_node().is_some()
}

/// Returns true if the node is a falsey literal (false, nil).
fn is_falsey_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_false_node().is_some() || node.as_nil_node().is_some()
}

/// Visitor to collect local variable write names from a node tree.
/// Stops at block boundaries because variables first assigned inside a block
/// are block-local and don't affect the while→loop scoping analysis.
struct LvarWriteCollector {
    names: Vec<Vec<u8>>,
}

impl LvarWriteCollector {
    fn add_name(&mut self, name: &[u8]) {
        let v = name.to_vec();
        if !self.names.contains(&v) {
            self.names.push(v);
        }
    }
}

impl<'pr> Visit<'pr> for LvarWriteCollector {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.add_name(node.name().as_slice());
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        self.add_name(node.name().as_slice());
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.add_name(node.name().as_slice());
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.add_name(node.name().as_slice());
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.add_name(node.name().as_slice());
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {}

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}

    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}

    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}

    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}

    fn visit_singleton_class_node(&mut self, _node: &ruby_prism::SingletonClassNode<'pr>) {}
}

/// Visitor to check if a variable is read after a given offset within the current scope.
struct ScopedLvarReadChecker<'a> {
    name: &'a [u8],
    after_offset: usize,
    found: bool,
}

impl ScopedLvarReadChecker<'_> {
    fn check_read(&mut self, name: &[u8], start_offset: usize) {
        if name == self.name && start_offset > self.after_offset {
            self.found = true;
        }
    }
}

impl<'pr> Visit<'pr> for ScopedLvarReadChecker<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        self.check_read(node.name().as_slice(), node.location().start_offset());
    }

    // Operator writes (x += 1) also read the variable
    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.check_read(node.name().as_slice(), node.location().start_offset());
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.check_read(node.name().as_slice(), node.location().start_offset());
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.check_read(node.name().as_slice(), node.location().start_offset());
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}

    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}

    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}

    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}

    fn visit_singleton_class_node(&mut self, _node: &ruby_prism::SingletonClassNode<'pr>) {}
}

/// Visitor to check if a variable is written before a given offset within the current scope.
/// Stops at block boundaries because block-local assignments don't create outer-scope variables.
struct ScopedLvarWriteChecker<'a> {
    name: &'a [u8],
    before_offset: usize,
    found: bool,
}

impl ScopedLvarWriteChecker<'_> {
    fn check_name_and_offset(&mut self, name: &[u8], end_offset: usize) {
        if name == self.name && end_offset < self.before_offset {
            self.found = true;
        }
    }
}

impl<'pr> Visit<'pr> for ScopedLvarWriteChecker<'_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.check_name_and_offset(node.name().as_slice(), node.location().end_offset());
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        self.check_name_and_offset(node.name().as_slice(), node.location().end_offset());
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        self.check_name_and_offset(node.name().as_slice(), node.location().end_offset());
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        self.check_name_and_offset(node.name().as_slice(), node.location().end_offset());
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        self.check_name_and_offset(node.name().as_slice(), node.location().end_offset());
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {}

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {}

    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {}

    fn visit_class_node(&mut self, _node: &ruby_prism::ClassNode<'pr>) {}

    fn visit_module_node(&mut self, _node: &ruby_prism::ModuleNode<'pr>) {}

    fn visit_singleton_class_node(&mut self, _node: &ruby_prism::SingletonClassNode<'pr>) {}
}

fn collect_lvar_writes(node: &ruby_prism::Node<'_>) -> Vec<Vec<u8>> {
    let mut collector = LvarWriteCollector { names: Vec::new() };
    collector.visit(node);
    collector.names
}

fn has_lvar_read_after(node: &ruby_prism::Node<'_>, name: &[u8], after_offset: usize) -> bool {
    let mut checker = ScopedLvarReadChecker {
        name,
        after_offset,
        found: false,
    };
    checker.visit(node);
    checker.found
}

fn has_lvar_write_before(node: &ruby_prism::Node<'_>, name: &[u8], before_offset: usize) -> bool {
    let mut checker = ScopedLvarWriteChecker {
        name,
        before_offset,
        found: false,
    };
    checker.visit(node);
    checker.found
}

/// Check if converting a while/until loop to `loop do` would break variable scoping.
/// Returns true if the offense should be suppressed.
fn would_break_scoping(
    scope: &ruby_prism::Node<'_>,
    loop_range: ruby_prism::Location<'_>,
    loop_stmts: Option<ruby_prism::StatementsNode<'_>>,
) -> bool {
    let stmts_node = match loop_stmts {
        Some(ref s) => s.as_node(),
        None => return false,
    };

    let vars_written_inside = collect_lvar_writes(&stmts_node);
    if vars_written_inside.is_empty() {
        return false;
    }

    for var_name in &vars_written_inside {
        let assigned_before = has_lvar_write_before(scope, var_name, loop_range.start_offset());
        if assigned_before {
            continue;
        }

        let referenced_after = has_lvar_read_after(scope, var_name, loop_range.end_offset());
        if referenced_after {
            return true;
        }
    }

    false
}

impl InfiniteLoopVisitor<'_, '_> {
    fn report_offense(&mut self, kw_loc: ruby_prism::Location<'_>) {
        let (line, column) = self.source.offset_to_line_col(kw_loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `Kernel#loop` for infinite loops.".to_string(),
        ));
    }
}

impl<'pr> Visit<'pr> for InfiniteLoopVisitor<'_, 'pr> {
    fn visit_program_node(&mut self, node: &ruby_prism::ProgramNode<'pr>) {
        self.scope_stack.push(node.statements().as_node());
        ruby_prism::visit_program_node(self, node);
        self.scope_stack.pop();
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        if let Some(body) = node.body() {
            self.scope_stack.push(body);
            ruby_prism::visit_def_node(self, node);
            self.scope_stack.pop();
        } else {
            ruby_prism::visit_def_node(self, node);
        }
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        if let Some(body) = node.body() {
            self.scope_stack.push(body);
            ruby_prism::visit_lambda_node(self, node);
            self.scope_stack.pop();
        } else {
            ruby_prism::visit_lambda_node(self, node);
        }
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        if let Some(body) = node.body() {
            self.scope_stack.push(body);
            ruby_prism::visit_class_node(self, node);
            self.scope_stack.pop();
        } else {
            ruby_prism::visit_class_node(self, node);
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        if let Some(body) = node.body() {
            self.scope_stack.push(body);
            ruby_prism::visit_module_node(self, node);
            self.scope_stack.pop();
        } else {
            ruby_prism::visit_module_node(self, node);
        }
    }

    fn visit_singleton_class_node(&mut self, node: &ruby_prism::SingletonClassNode<'pr>) {
        if let Some(body) = node.body() {
            self.scope_stack.push(body);
            ruby_prism::visit_singleton_class_node(self, node);
            self.scope_stack.pop();
        } else {
            ruby_prism::visit_singleton_class_node(self, node);
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        if let Some(body) = node.body() {
            self.scope_stack.push(body);
            ruby_prism::visit_block_node(self, node);
            self.scope_stack.pop();
        } else {
            ruby_prism::visit_block_node(self, node);
        }
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        let should_report = if let Some(scope) = self.scope_stack.last() {
            is_truthy_literal(&node.predicate())
                && !would_break_scoping(scope, node.location(), node.statements())
        } else {
            false
        };
        if should_report {
            self.report_offense(node.keyword_loc());
        }
        ruby_prism::visit_while_node(self, node);
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        let should_report = if let Some(scope) = self.scope_stack.last() {
            is_falsey_literal(&node.predicate())
                && !would_break_scoping(scope, node.location(), node.statements())
        } else {
            false
        };
        if should_report {
            self.report_offense(node.keyword_loc());
        }
        ruby_prism::visit_until_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InfiniteLoop, "cops/style/infinite_loop");
}
