use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// ## Corpus investigation (2026-03-08)
///
/// FP=187, FN=7. Three root causes for FPs:
///
/// 1. **Path matching issue (~64 FPs):** The `default_include` glob `app/helpers/**/*.rb`
///    matched paths like `test/unit/app/helpers/admin/helper_test.rb` because `app/helpers/`
///    appears as a substring. This is a config/engine-level issue with how Include globs are
///    anchored — not fixable in the cop itself. (Resolved separately if needed.)
///
/// 2. **Missing nested class skip (~40-50 FPs):** Per RuboCop, instance variables inside
///    any class definition within a helper file should NOT be flagged — the ivar belongs
///    to the class, not the helper module. Fixed by using a visitor that tracks `in_class`
///    depth and skips ivars when inside any class.
///
/// 3. **Missing memoization skip (~20 FPs):** Per RuboCop, `@cache ||= expr`
///    (`InstanceVariableOrWriteNode` in Prism / `ivasgn` under `or_asgn` in Parser)
///    is a memoization pattern and should not be flagged. Fixed by not visiting
///    `InstanceVariableOrWriteNode`.
///
/// FN=7 were caused by missing node types: `InstanceVariableOperatorWriteNode` (`@x += 1`),
/// `InstanceVariableAndWriteNode` (`@x &&= false`), and `InstanceVariableTargetNode`
/// (`@a, @b = vals`). These are now handled via the visitor.
///
/// The cop now uses `check_source` with a `Visit` implementation instead of `check_node`,
/// allowing proper tracking of class nesting depth.
pub struct HelperInstanceVariable;

impl Cop for HelperInstanceVariable {
    fn name(&self) -> &'static str {
        "Rails/HelperInstanceVariable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["app/helpers/**/*.rb"]
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
        let mut visitor = HelperIvarVisitor {
            cop: self,
            source,
            diagnostics: Vec::new(),
            class_depth: 0,
        };
        visitor.visit(&parse_result.node());
        diagnostics.extend(visitor.diagnostics);
    }
}

struct HelperIvarVisitor<'a> {
    cop: &'a HelperInstanceVariable,
    source: &'a SourceFile,
    diagnostics: Vec<Diagnostic>,
    /// Track nesting depth inside class definitions. When > 0, ivars belong
    /// to the class and should not be flagged.
    class_depth: usize,
}

impl HelperIvarVisitor<'_> {
    fn add_offense(&mut self, offset: usize) {
        if self.class_depth > 0 {
            return;
        }
        let (line, column) = self.source.offset_to_line_col(offset);
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Do not use instance variables in helpers.".to_string(),
        ));
    }
}

impl<'pr> Visit<'pr> for HelperIvarVisitor<'_> {
    fn visit_class_node(&mut self, node: &ruby_prism::ClassNode<'pr>) {
        self.class_depth += 1;
        if let Some(body) = node.body() {
            self.visit(&body);
        }
        self.class_depth -= 1;
    }

    fn visit_instance_variable_read_node(
        &mut self,
        node: &ruby_prism::InstanceVariableReadNode<'pr>,
    ) {
        self.add_offense(node.location().start_offset());
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.add_offense(node.location().start_offset());
        // Visit the value expression
        self.visit(&node.value());
    }

    fn visit_instance_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableOperatorWriteNode<'pr>,
    ) {
        self.add_offense(node.location().start_offset());
        self.visit(&node.value());
    }

    fn visit_instance_variable_and_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableAndWriteNode<'pr>,
    ) {
        self.add_offense(node.location().start_offset());
        self.visit(&node.value());
    }

    fn visit_instance_variable_target_node(
        &mut self,
        node: &ruby_prism::InstanceVariableTargetNode<'pr>,
    ) {
        self.add_offense(node.location().start_offset());
    }

    // Deliberately NOT implementing visit_instance_variable_or_write_node —
    // `@x ||= expr` is a memoization pattern and should not be flagged per RuboCop.
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        HelperInstanceVariable,
        "cops/rails/helper_instance_variable"
    );
}
