use crate::cop::shared::node_type::{IF_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for literal assignments used anywhere inside a conditional expression.
///
/// ## Corpus investigation (2026-03-10)
///
/// Corpus oracle reported FP=0, FN=47.
///
/// FN:
/// - The implementation only recursed through top-level `&&` / `||`, so assignments buried in
///   comparisons and method-call arguments, such as `(e = 0) == 0` and `include?(e = 0)`, were
///   missed. A condition visitor is needed instead of hand-walking just boolean operators.
/// - Ruby also warns on literal-only array and hash assignments. Prism splits hashes into
///   `HashNode` and `KeywordHashNode`, so the literal check must recurse through both.
/// - A blanket `BlockNode` skip was too aggressive. RuboCop still flags literal assignments
///   inside blocks that participate in the condition value, such as
///   `if validate(resource) { hashed = true; ... }`.
///
/// Current corpus rerun after these fixes: expected 50, actual 53. The remaining raw excess is
/// explained by `jruby` file-drop noise (RuboCop parser crashes) plus one `rails/sprockets`
/// count mismatch that did not reproduce from the generic local fixture
/// `if File.exist?(path = "./.sprocketsrc")`.
///
/// ## Corpus investigation (2026-03-27)
///
/// Corpus oracle reported FP=1, FN=3.
///
/// FN:
/// - `unless respond_to?(type_reader = :"#{type}_attrs")` and
///   `instance_variable_defined?(ivar = :"@#{type}_fields")` were missed because
///   `InterpolatedSymbolNode` (`dsym`) was not treated as literal.
/// - `time_range = active_duty_start..active_duty_end` inside a conditional block was missed
///   because `RangeNode` was not treated as literal for this cop.
///
/// Fix:
/// - Extend `is_literal()` to treat `InterpolatedSymbolNode` and `RangeNode` as literals,
///   matching RuboCop's `all_literals?` behavior for this cop while still excluding
///   interpolated string/xstring (`dstr`/`xstr`).
///
/// ## Corpus investigation (2026-04-01)
///
/// Corpus oracle reported FP=1, FN=0.
///
/// FP:
/// - `gr = '\A[\n    ]'` (a single-quoted string spanning two physical lines) was flagged
///   because Prism represents it as a `StringNode`, but CRuby's parser gem represents
///   multi-line strings as `:dstr`, which RuboCop's `all_literals?` explicitly excludes.
///
/// Fix:
/// - In `is_literal()`, treat `StringNode`s whose source contains a newline as non-literal,
///   matching the parser gem's `:dstr` representation for multi-line strings.
pub struct LiteralAssignmentInCondition;

impl Cop for LiteralAssignmentInCondition {
    fn name(&self) -> &'static str {
        "Lint/LiteralAssignmentInCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[IF_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Get the condition from if/while/until
        let predicate = if let Some(if_node) = node.as_if_node() {
            Some(if_node.predicate())
        } else if let Some(unless_node) = node.as_unless_node() {
            Some(unless_node.predicate())
        } else if let Some(while_node) = node.as_while_node() {
            Some(while_node.predicate())
        } else {
            node.as_until_node()
                .map(|until_node| until_node.predicate())
        };

        let predicate = match predicate {
            Some(p) => p,
            None => return,
        };

        let mut finder = LiteralAssignmentFinder {
            cop: self,
            source,
            diagnostics,
        };
        finder.visit(&predicate);
    }
}

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    if node.as_interpolated_string_node().is_some()
        || node.as_interpolated_x_string_node().is_some()
    {
        return false;
    }

    if let Some(array) = node.as_array_node() {
        return array.elements().iter().all(|element| is_literal(&element));
    }

    if let Some(hash) = node.as_hash_node() {
        return hash_elements_are_literals(hash.elements().iter());
    }

    if let Some(hash) = node.as_keyword_hash_node() {
        return hash_elements_are_literals(hash.elements().iter());
    }

    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node
            .as_string_node()
            .is_some_and(|_| !node.location().as_slice().contains(&b'\n'))
        || node.as_symbol_node().is_some()
        || node.as_interpolated_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_regular_expression_node().is_some()
        || node.as_range_node().is_some()
}

fn hash_elements_are_literals<'pr>(
    mut elements: impl Iterator<Item = ruby_prism::Node<'pr>>,
) -> bool {
    elements.all(|element| {
        let Some(assoc) = element.as_assoc_node() else {
            return false;
        };

        is_literal(&assoc.key()) && is_literal(&assoc.value())
    })
}

struct LiteralAssignmentFinder<'a, 'b> {
    cop: &'a LiteralAssignmentInCondition,
    source: &'a SourceFile,
    diagnostics: &'b mut Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for LiteralAssignmentFinder<'_, '_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        self.check_assignment(&node.as_node(), &node.value());
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_instance_variable_write_node(
        &mut self,
        node: &ruby_prism::InstanceVariableWriteNode<'pr>,
    ) {
        self.check_assignment(&node.as_node(), &node.value());
        ruby_prism::visit_instance_variable_write_node(self, node);
    }

    fn visit_class_variable_write_node(&mut self, node: &ruby_prism::ClassVariableWriteNode<'pr>) {
        self.check_assignment(&node.as_node(), &node.value());
        ruby_prism::visit_class_variable_write_node(self, node);
    }

    fn visit_global_variable_write_node(
        &mut self,
        node: &ruby_prism::GlobalVariableWriteNode<'pr>,
    ) {
        self.check_assignment(&node.as_node(), &node.value());
        ruby_prism::visit_global_variable_write_node(self, node);
    }

    fn visit_constant_write_node(&mut self, node: &ruby_prism::ConstantWriteNode<'pr>) {
        self.check_assignment(&node.as_node(), &node.value());
        ruby_prism::visit_constant_write_node(self, node);
    }
}

impl LiteralAssignmentFinder<'_, '_> {
    fn check_assignment(&mut self, node: &ruby_prism::Node<'_>, rhs: &ruby_prism::Node<'_>) {
        if !is_literal(rhs) {
            return;
        }

        let rhs_loc = rhs.location();
        let rhs_src = self
            .source
            .byte_slice(rhs_loc.start_offset(), rhs_loc.end_offset(), "?");
        let loc = node.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        self.diagnostics.push(self.cop.diagnostic(
            self.source,
            line,
            column,
            format!(
                "Don't use literal assignment `= {rhs_src}` in conditional, should be `==` or non-literal operand."
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        LiteralAssignmentInCondition,
        "cops/lint/literal_assignment_in_condition"
    );
}
