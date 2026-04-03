use crate::cop::shared::constant_predicates;
use crate::cop::shared::util::{
    RSPEC_DEFAULT_INCLUDE, is_rspec_example_group, is_rspec_shared_group,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Flags constant assignments (`CONST = ...`), class definitions, and module
/// definitions inside RSpec example groups. These leak into the global namespace.
///
/// **Root cause of 2,109 FNs (round 1):** The previous implementation only scanned direct
/// statements in example group block bodies. Constants/classes/modules nested inside
/// control structures (if/unless/case/begin/etc.) were missed.
///
/// **Fix (round 1):** Rewrote to use `check_source` with a visitor that tracks example group
/// depth. When visiting ConstantWriteNode, ClassNode, or ModuleNode while inside
/// any example group (depth > 0), flags the offense. This matches RuboCop's
/// ancestor-checking approach: `node.each_ancestor(:block).any? { |a| spec_group?(a) }`.
///
/// **Root cause of 2,122 FNs (round 2):** Two issues:
/// 1. Missing node types: `ConstantOrWriteNode` (`CONST ||= val`),
///    `ConstantAndWriteNode` (`CONST &&= val`), and `ConstantOperatorWriteNode`
///    (`CONST += val`) were not handled. In the Parser gem these are all `casgn` nodes.
/// 2. Not recursing into class/module bodies: RuboCop's `inside_describe_block?` checks
///    ancestors for `:block` nodes (classes/modules aren't blocks), so `CONST = val` inside
///    a class inside an example group is still flagged. The previous implementation skipped
///    class/module body recursion entirely.
///
/// **Fix (round 2):** Added visitors for all constant write node types. Changed class/module
/// visitors to recurse into their bodies so constants inside them are also detected.
///
/// **Root cause of FN=300 (round 3):** `visit_module_node` and `visit_class_node` only
/// recursed into their bodies when `example_group_depth > 0`. Spec files that wrap
/// describe blocks inside module/class declarations (at depth 0) had their inner
/// describe blocks completely skipped. Fix: always recurse into module/class bodies,
/// only emit module/class offenses when depth > 0.
///
/// **Root cause of FN=23 (round 4):** Constant assignments used as arguments to `describe`
/// calls (e.g., `describe MyConst = SomeModule::SomeClass do`) were not visited because
/// only the block body was traversed at incremented depth, not the call arguments.
/// Fix: also visit the call's arguments after incrementing `example_group_depth`.
///
/// **Root cause of FN=12 (round 5):** Two missing node types:
/// 1. `MultiWriteNode` with `ConstantTargetNode` targets (6 FNs): Patterns like
///    `CONST_A, CONST_B = 1, 2` create MultiWriteNode with ConstantTargetNode in `lefts()`.
///    Also handles splatted constant targets in the `rest()` position.
/// 2. `ForNode` with constant iterator (2 FNs): `for CONST in collection` uses a
///    ConstantTargetNode as the loop index variable.
///
/// **Root cause of FN=4 (round 6):** Constant write visitors (`visit_constant_write_node`, etc.)
/// did not recurse into their children. When `Custom_enum = Class.new do...end` was visited,
/// the outer `ConstantWriteNode` was flagged but the `Class.new` block body (containing
/// `ToNativeMap` and `FromNativeMap` assignments) was never traversed.
/// Fix: added `ruby_prism::visit_constant_*_write_node(self, node)` calls to all four
/// constant write visitors so they recurse into their value expressions.
pub struct LeakyConstantDeclaration;

impl Cop for LeakyConstantDeclaration {
    fn name(&self) -> &'static str {
        "RSpec/LeakyConstantDeclaration"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
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
        let mut visitor = LeakyVisitor {
            source,
            cop: self,
            example_group_depth: 0,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct LeakyVisitor<'a> {
    source: &'a SourceFile,
    cop: &'a LeakyConstantDeclaration,
    /// Tracks how deep we are inside example group blocks. > 0 means inside.
    example_group_depth: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> LeakyVisitor<'a> {
    fn is_example_group_call(&self, call: &ruby_prism::CallNode<'_>) -> bool {
        let method_name = call.name().as_slice();
        if let Some(recv) = call.receiver() {
            constant_predicates::constant_short_name(&recv).is_some_and(|n| n == b"RSpec")
                && (is_rspec_example_group(method_name) || is_rspec_shared_group(method_name))
        } else {
            is_rspec_example_group(method_name) || is_rspec_shared_group(method_name)
        }
    }
}

impl Visit<'_> for LeakyVisitor<'_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'_>) {
        if self.is_example_group_call(node) {
            if let Some(block) = node.block() {
                if let Some(block_node) = block.as_block_node() {
                    self.example_group_depth += 1;
                    // Visit arguments with incremented depth — constant writes
                    // in describe arguments (e.g., `describe MyConst = Foo do`)
                    // are leaky too.
                    if let Some(args) = node.arguments() {
                        for arg in args.arguments().iter() {
                            self.visit(&arg);
                        }
                    }
                    // Visit block body with incremented depth.
                    if let Some(body) = block_node.body() {
                        self.visit(&body);
                    }
                    self.example_group_depth -= 1;
                    return;
                }
            }
        }
        // For non-example-group calls, visit children normally
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'_>) {
        if self.example_group_depth > 0 {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Stub constant instead of declaring explicitly.".to_string(),
            ));
        }
        // Recurse into the value — it may contain blocks (e.g., Class.new do...end)
        // with more constant assignments inside.
        ruby_prism::visit_constant_write_node(self, node);
    }

    fn visit_constant_or_write_node(&mut self, node: &ruby_prism::ConstantOrWriteNode<'_>) {
        if self.example_group_depth > 0 {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Stub constant instead of declaring explicitly.".to_string(),
            ));
        }
        ruby_prism::visit_constant_or_write_node(self, node);
    }

    fn visit_constant_and_write_node(&mut self, node: &ruby_prism::ConstantAndWriteNode<'_>) {
        if self.example_group_depth > 0 {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Stub constant instead of declaring explicitly.".to_string(),
            ));
        }
        ruby_prism::visit_constant_and_write_node(self, node);
    }

    fn visit_constant_operator_write_node(
        &mut self,
        node: &ruby_prism::ConstantOperatorWriteNode<'_>,
    ) {
        if self.example_group_depth > 0 {
            let loc = node.location();
            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
            self.diagnostics.push(self.cop.diagnostic(
                self.source,
                line,
                column,
                "Stub constant instead of declaring explicitly.".to_string(),
            ));
        }
        ruby_prism::visit_constant_operator_write_node(self, node);
    }

    fn visit_multi_write_node(&mut self, node: &ruby_prism::MultiWriteNode<'_>) {
        if self.example_group_depth > 0 {
            // Check lefts for ConstantTargetNode targets
            for target in node.lefts().iter() {
                if let Some(ct) = target.as_constant_target_node() {
                    let loc = ct.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Stub constant instead of declaring explicitly.".to_string(),
                    ));
                }
            }
            // Check rights (not needed — rights are values, not targets)
            // Check rest for splatted constant target
            if let Some(rest) = node.rest() {
                if let Some(ct) = rest.as_constant_target_node() {
                    let loc = ct.location();
                    let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                    self.diagnostics.push(self.cop.diagnostic(
                        self.source,
                        line,
                        column,
                        "Stub constant instead of declaring explicitly.".to_string(),
                    ));
                } else if let Some(splat) = rest.as_splat_node() {
                    if let Some(expr) = splat.expression() {
                        if let Some(ct) = expr.as_constant_target_node() {
                            let loc = ct.location();
                            let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                            self.diagnostics.push(self.cop.diagnostic(
                                self.source,
                                line,
                                column,
                                "Stub constant instead of declaring explicitly.".to_string(),
                            ));
                        }
                    }
                }
            }
        }
        // Recurse into values in case they contain nested example groups
        ruby_prism::visit_multi_write_node(self, node);
    }

    fn visit_for_node(&mut self, node: &ruby_prism::ForNode<'_>) {
        if self.example_group_depth > 0 {
            let index = node.index();
            if let Some(ct) = index.as_constant_target_node() {
                let loc = ct.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Stub constant instead of declaring explicitly.".to_string(),
                ));
            }
        }
        // Recurse into body in case it contains nested example groups
        ruby_prism::visit_for_node(self, node);
    }

    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'_>) {
        if self.example_group_depth > 0 {
            let const_path = node.constant_path();
            // Only flag bare class names (ConstantReadNode), not qualified ones.
            // constant_path_node (Foo::Bar, self::Bar, ::Bar) is intentionally
            // excluded — qualified constants don't leak in the same way.
            if const_path.as_constant_read_node().is_some() {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Stub class constant instead of declaring explicitly.".to_string(),
                ));
            }
        }
        // Always recurse into class body regardless of depth. At depth 0, the class itself
        // is not flagged, but describe blocks nested inside it still need to be found.
        // RuboCop's `inside_describe_block?` checks ancestor blocks (classes aren't blocks),
        // so constants inside a class inside an example group are still flagged.
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }

    fn visit_module_node(&mut self, node: &ruby_prism::ModuleNode<'_>) {
        if self.example_group_depth > 0 {
            let const_path = node.constant_path();
            if const_path.as_constant_read_node().is_some() {
                let loc = node.location();
                let (line, column) = self.source.offset_to_line_col(loc.start_offset());
                self.diagnostics.push(self.cop.diagnostic(
                    self.source,
                    line,
                    column,
                    "Stub module constant instead of declaring explicitly.".to_string(),
                ));
            }
        }
        // Always recurse into module body regardless of depth. At depth 0, the module itself
        // is not flagged, but describe blocks nested inside it still need to be found.
        if let Some(body) = node.body() {
            self.visit(&body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        LeakyConstantDeclaration,
        "cops/rspec/leaky_constant_declaration"
    );
}
