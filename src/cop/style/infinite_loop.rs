use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Style/InfiniteLoop
///
/// ## Investigation findings
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
/// exactly once and keeps the existing scoping exemption logic unchanged.
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
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct InfiniteLoopVisitor<'a> {
    cop: &'a InfiniteLoop,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
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
struct LvarWriteCollector {
    names: Vec<Vec<u8>>,
}

impl<'pr> Visit<'pr> for LvarWriteCollector {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        let name = node.name().as_slice().to_vec();
        if !self.names.contains(&name) {
            self.names.push(name);
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        let name = node.name().as_slice().to_vec();
        if !self.names.contains(&name) {
            self.names.push(name);
        }
    }
}

/// Visitor to check if any of the given variable names are read.
struct LvarReadChecker<'a> {
    names: &'a [Vec<u8>],
    found: bool,
}

impl<'pr> Visit<'pr> for LvarReadChecker<'_> {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        if self.names.contains(&node.name().as_slice().to_vec()) {
            self.found = true;
        }
    }
}

/// Visitor to check if a specific variable name is written.
struct LvarWriteChecker<'a> {
    name: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for LvarWriteChecker<'_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_target_node(
        &mut self,
        node: &ruby_prism::LocalVariableTargetNode<'pr>,
    ) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
    }
}

fn collect_lvar_writes(node: &ruby_prism::Node<'_>) -> Vec<Vec<u8>> {
    let mut collector = LvarWriteCollector { names: Vec::new() };
    collector.visit(node);
    collector.names
}

fn has_lvar_read(node: &ruby_prism::Node<'_>, names: &[Vec<u8>]) -> bool {
    let mut checker = LvarReadChecker {
        names,
        found: false,
    };
    checker.visit(node);
    checker.found
}

fn has_lvar_write(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    let mut checker = LvarWriteChecker { name, found: false };
    checker.visit(node);
    checker.found
}

/// Check if converting a while/until loop to `loop do` would break variable scoping.
/// Returns true if the offense should be suppressed.
fn would_break_scoping(
    siblings: &[ruby_prism::Node<'_>],
    loop_index: usize,
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
        // Check if variable is assigned before the loop
        let assigned_before = siblings[..loop_index]
            .iter()
            .any(|s| has_lvar_write(s, var_name));
        if assigned_before {
            continue;
        }

        // Check if variable is referenced after the loop
        let referenced_after = siblings[loop_index + 1..]
            .iter()
            .any(|s| has_lvar_read(s, std::slice::from_ref(var_name)));
        if referenced_after {
            return true;
        }
    }

    false
}

impl InfiniteLoopVisitor<'_> {
    fn check_statements(&mut self, stmts: &[ruby_prism::Node<'_>]) {
        for (i, stmt) in stmts.iter().enumerate() {
            if let Some(while_node) = stmt.as_while_node() {
                if is_truthy_literal(&while_node.predicate())
                    && !would_break_scoping(stmts, i, while_node.statements())
                {
                    let kw_loc = while_node.keyword_loc();
                    let (line, column) = self.source.offset_to_line_col(kw_loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use `Kernel#loop` for infinite loops.".to_string(),
                    ));
                }
            } else if let Some(until_node) = stmt.as_until_node() {
                if is_falsey_literal(&until_node.predicate())
                    && !would_break_scoping(stmts, i, until_node.statements())
                {
                    let kw_loc = until_node.keyword_loc();
                    let (line, column) = self.source.offset_to_line_col(kw_loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Use `Kernel#loop` for infinite loops.".to_string(),
                    ));
                }
            }
        }
    }
}

impl<'pr> Visit<'pr> for InfiniteLoopVisitor<'_> {
    fn visit_statements_node(&mut self, node: &ruby_prism::StatementsNode<'pr>) {
        let children: Vec<_> = node.body().iter().collect();
        self.check_statements(&children);
        ruby_prism::visit_statements_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InfiniteLoop, "cops/style/infinite_loop");
}
