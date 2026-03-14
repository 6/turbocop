use std::collections::HashSet;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for constant reassignments.
///
/// Emulates Ruby's runtime warning "already initialized constant X" when a constant
/// is reassigned in the same file and namespace using `NAME = value` syntax.
///
/// ## Implementation notes (corpus investigation)
///
/// Key differences vs initial implementation that caused FP=6, FN=8:
///
/// 1. **ConstantPathWriteNode**: Must handle `A::FOO = :bar`, `self::FOO = :bar`,
///    `::FOO = :bar` in addition to simple `FOO = :bar` (ConstantWriteNode).
///
/// 2. **fixed_constant_path?**: Skip assignments with variable paths like
///    `lvar::FOO = 1` where the receiver is not a constant, cbase, or self.
///
/// 3. **simple_assignment?**: Only flag assignments in "simple" contexts — directly
///    inside class/module body, top-level, or nested in other constant assignments.
///    Assignments inside conditionals, blocks, methods, rescue, etc. are skipped.
///    Class/module boundaries reset the context (a class inside an `unless` still
///    has its body treated as simple).
///
/// 4. **remove_const tracking**: `remove_const :FOO` or `remove_const 'FOO'`
///    removes the constant from the seen set, allowing re-assignment without offense.
/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=6, FN=0. All 6 FPs are config/exclude differences:
///
/// - jruby (2): spec/ruby/fixtures/constants.rb — intentional reassignment
///   test fixtures excluded by project .rubocop.yml
/// - natalie (2): same file (mirrors jruby's spec suite)
/// - rufo (2): .rb.spec formatter test files excluded by project config
///
/// Cop logic correctly detects reassignment; the files are excluded by the
/// target project's RuboCop configuration. No cop logic bugs.
pub struct ConstantReassignment;

impl Cop for ConstantReassignment {
    fn name(&self) -> &'static str {
        "Lint/ConstantReassignment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut visitor = ConstantReassignmentVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            seen_constants: HashSet::new(),
            namespace_stack: Vec::new(),
            non_simple_depth: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct ConstantReassignmentVisitor<'a, 'src> {
    cop: &'a ConstantReassignment,
    source: &'src SourceFile,
    diagnostics: Vec<Diagnostic>,
    seen_constants: HashSet<String>,
    namespace_stack: Vec<String>,
    /// Tracks nesting depth inside non-simple contexts (conditionals, blocks,
    /// methods, rescue, etc.). When > 0, constant writes are skipped because
    /// they may be conditional first-time assignments.
    /// Class/module boundaries reset this to 0 (they create a new simple context).
    non_simple_depth: usize,
}

impl ConstantReassignmentVisitor<'_, '_> {
    fn fully_qualified_name(&self, name: &str) -> String {
        let mut parts = Vec::new();
        for ns in &self.namespace_stack {
            parts.push(ns.as_str());
        }
        parts.push(name);
        format!("::{}", parts.join("::"))
    }

    /// Build a fully qualified name from a ConstantPathNode target.
    /// For `A::B::FOO`, walks the path to build `::A::B::FOO`.
    /// For `::FOO`, returns `::FOO`.
    /// For `self::FOO` inside class A, returns `::A::FOO`.
    /// Returns None for variable paths like `lvar::FOO`.
    fn fqn_from_constant_path(&self, node: &ruby_prism::ConstantPathNode<'_>) -> Option<String> {
        let (segments, is_absolute) = self.collect_path_segments(node)?;

        if is_absolute {
            Some(format!("::{}", segments.join("::")))
        } else {
            // Relative path — prepend namespace stack
            let mut parts: Vec<&str> = self.namespace_stack.iter().map(|s| s.as_str()).collect();
            for seg in &segments {
                parts.push(seg.as_str());
            }
            Some(format!("::{}", parts.join("::")))
        }
    }

    /// Collect path segments from a ConstantPathNode into a vector, returning
    /// None if any part of the path is a variable (non-fixed).
    /// Also returns whether the path is absolute (rooted at cbase `::`) or
    /// relative to `self`.
    fn collect_path_segments(
        &self,
        node: &ruby_prism::ConstantPathNode<'_>,
    ) -> Option<(Vec<String>, bool)> {
        let seg_name = node
            .name()
            .map(|n| std::str::from_utf8(n.as_slice()).unwrap_or("").to_string())
            .unwrap_or_default();

        if let Some(parent) = node.parent() {
            if let Some(parent_const) = parent.as_constant_read_node() {
                let parent_name = std::str::from_utf8(parent_const.name().as_slice()).unwrap_or("");
                Some((vec![parent_name.to_string(), seg_name], false))
            } else if let Some(parent_path) = parent.as_constant_path_node() {
                let (mut segments, is_absolute) = self.collect_path_segments(&parent_path)?;
                segments.push(seg_name);
                Some((segments, is_absolute))
            } else if parent.as_self_node().is_some() {
                Some((vec![seg_name], false))
            } else {
                // Variable path — not fixed
                None
            }
        } else {
            // ::X — cbase, absolute
            Some((vec![seg_name], true))
        }
    }

    /// Record or check a constant assignment, emitting a diagnostic if it's a reassignment.
    fn record_constant(&mut self, fqn: String, name: &str, start_offset: usize, end_offset: usize) {
        if !self.seen_constants.insert(fqn) {
            let (line, column) = self.source.offset_to_line_col(start_offset);
            // Calculate the length of the offense span
            let (end_line, end_column) = self.source.offset_to_line_col(end_offset);
            let _ = (end_line, end_column); // used for span but diagnostic uses start only
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                format!("Constant `{name}` is already assigned in this namespace."),
            ));
        }
    }

    /// Remove a constant from the seen set (for remove_const tracking).
    fn remove_constant(&mut self, name: &str) {
        let fqn = self.fully_qualified_name(name);
        self.seen_constants.remove(&fqn);
    }
}

impl<'pr> Visit<'pr> for ConstantReassignmentVisitor<'_, '_> {
    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        if self.non_simple_depth == 0 {
            let name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
            let fqn = self.fully_qualified_name(name);
            let loc = node.name_loc();
            self.record_constant(fqn, name, loc.start_offset(), loc.end_offset());
        }

        ruby_prism::visit_constant_write_node(self, node);
    }

    fn visit_constant_path_write_node(&mut self, node: &ruby_prism::ConstantPathWriteNode<'pr>) {
        if self.non_simple_depth == 0 {
            let target = node.target();
            if let Some(fqn) = self.fqn_from_constant_path(&target) {
                let child_name = target
                    .name()
                    .map(|n| std::str::from_utf8(n.as_slice()).unwrap_or(""))
                    .unwrap_or("");
                // Use the full node location for the offense
                let loc = node.location();
                self.record_constant(fqn, child_name, loc.start_offset(), loc.end_offset());
            }
            // If fqn_from_constant_path returns None, it's a variable path — skip
        }

        ruby_prism::visit_constant_path_write_node(self, node);
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        // Class/module boundaries reset non_simple_depth — a class body is always
        // a simple context, even if the class itself is inside a conditional.
        let saved_depth = self.non_simple_depth;
        self.non_simple_depth = 0;
        self.namespace_stack.push(name);
        ruby_prism::visit_class_node(self, node);
        self.namespace_stack.pop();
        self.non_simple_depth = saved_depth;
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'pr>) {
        let name = std::str::from_utf8(node.name().as_slice())
            .unwrap_or("")
            .to_string();
        let saved_depth = self.non_simple_depth;
        self.non_simple_depth = 0;
        self.namespace_stack.push(name);
        ruby_prism::visit_module_node(self, node);
        self.namespace_stack.pop();
        self.non_simple_depth = saved_depth;
    }

    // Non-simple contexts: constant assignments inside these are not
    // unconditional reassignments.

    fn visit_if_node(&mut self, node: &ruby_prism::IfNode<'pr>) {
        self.non_simple_depth += 1;
        ruby_prism::visit_if_node(self, node);
        self.non_simple_depth -= 1;
    }

    fn visit_unless_node(&mut self, node: &ruby_prism::UnlessNode<'pr>) {
        self.non_simple_depth += 1;
        ruby_prism::visit_unless_node(self, node);
        self.non_simple_depth -= 1;
    }

    fn visit_case_node(&mut self, node: &ruby_prism::CaseNode<'pr>) {
        self.non_simple_depth += 1;
        ruby_prism::visit_case_node(self, node);
        self.non_simple_depth -= 1;
    }

    fn visit_begin_node(&mut self, node: &ruby_prism::BeginNode<'pr>) {
        if node.rescue_clause().is_some() {
            self.non_simple_depth += 1;
            ruby_prism::visit_begin_node(self, node);
            self.non_simple_depth -= 1;
        } else {
            ruby_prism::visit_begin_node(self, node);
        }
    }

    fn visit_block_node(&mut self, node: &ruby_prism::BlockNode<'pr>) {
        self.non_simple_depth += 1;
        ruby_prism::visit_block_node(self, node);
        self.non_simple_depth -= 1;
    }

    fn visit_def_node(&mut self, node: &ruby_prism::DefNode<'pr>) {
        self.non_simple_depth += 1;
        ruby_prism::visit_def_node(self, node);
        self.non_simple_depth -= 1;
    }

    fn visit_lambda_node(&mut self, node: &ruby_prism::LambdaNode<'pr>) {
        self.non_simple_depth += 1;
        ruby_prism::visit_lambda_node(self, node);
        self.non_simple_depth -= 1;
    }

    fn visit_while_node(&mut self, node: &ruby_prism::WhileNode<'pr>) {
        self.non_simple_depth += 1;
        ruby_prism::visit_while_node(self, node);
        self.non_simple_depth -= 1;
    }

    fn visit_until_node(&mut self, node: &ruby_prism::UntilNode<'pr>) {
        self.non_simple_depth += 1;
        ruby_prism::visit_until_node(self, node);
        self.non_simple_depth -= 1;
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'pr>) {
        self.non_simple_depth += 1;
        ruby_prism::visit_for_node(self, node);
        self.non_simple_depth -= 1;
    }

    // Track remove_const calls to allow re-assignment after removal.
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = std::str::from_utf8(node.name().as_slice()).unwrap_or("");
        if method_name == "remove_const" {
            if let Some(args) = node.arguments() {
                let mut arg_iter = args.arguments().iter();
                if let Some(first_arg) = arg_iter.next() {
                    // Only handle single-argument calls
                    if arg_iter.next().is_none() {
                        // remove_const :FOO
                        if let Some(sym) = first_arg.as_symbol_node() {
                            let const_name = std::str::from_utf8(sym.unescaped()).unwrap_or("");
                            self.remove_constant(const_name);
                        }
                        // remove_const 'FOO'
                        else if let Some(str_node) = first_arg.as_string_node() {
                            let const_name =
                                std::str::from_utf8(str_node.unescaped()).unwrap_or("");
                            self.remove_constant(const_name);
                        }
                    }
                }
            }
        }

        ruby_prism::visit_call_node(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ConstantReassignment, "cops/lint/constant_reassignment");
}
