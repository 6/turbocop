use crate::cop::node_type::{
    AND_NODE, CLASS_VARIABLE_WRITE_NODE, CONSTANT_WRITE_NODE, FALSE_NODE, FLOAT_NODE,
    GLOBAL_VARIABLE_WRITE_NODE, IF_NODE, INSTANCE_VARIABLE_WRITE_NODE, INTEGER_NODE,
    LOCAL_VARIABLE_WRITE_NODE, NIL_NODE, OR_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE,
    SYMBOL_NODE, TRUE_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct LiteralAssignmentInCondition;

impl Cop for LiteralAssignmentInCondition {
    fn name(&self) -> &'static str {
        "Lint/LiteralAssignmentInCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            AND_NODE,
            CLASS_VARIABLE_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            FALSE_NODE,
            FLOAT_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            IF_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            INTEGER_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
            NIL_NODE,
            OR_NODE,
            REGULAR_EXPRESSION_NODE,
            STRING_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
            UNLESS_NODE,
            UNTIL_NODE,
            WHILE_NODE,
        ]
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

        check_node_for_literal_assignment(self, source, &predicate, diagnostics);
    }
}

fn check_node_for_literal_assignment(
    cop: &LiteralAssignmentInCondition,
    source: &SourceFile,
    node: &ruby_prism::Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check if this is an assignment with a literal RHS
    let rhs = if let Some(lv) = node.as_local_variable_write_node() {
        Some(lv.value())
    } else if let Some(iv) = node.as_instance_variable_write_node() {
        Some(iv.value())
    } else if let Some(cv) = node.as_class_variable_write_node() {
        Some(cv.value())
    } else if let Some(gv) = node.as_global_variable_write_node() {
        Some(gv.value())
    } else {
        node.as_constant_write_node().map(|cw| cw.value())
    };

    if let Some(rhs) = rhs {
        if is_literal(&rhs) {
            let rhs_loc = rhs.location();
            let rhs_src = source.byte_slice(rhs_loc.start_offset(), rhs_loc.end_offset(), "?");

            // Offense location is at the operator (=)
            // We report at the assignment node start for simplicity
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(cop.diagnostic(
                source,
                line,
                column,
                format!(
                    "Don't use literal assignment `= {rhs_src}` in conditional, should be `==` or non-literal operand."
                ),
            ));
        }
    }

    // Recurse into and/or nodes within the condition
    if let Some(and_node) = node.as_and_node() {
        check_node_for_literal_assignment(cop, source, &and_node.left(), diagnostics);
        check_node_for_literal_assignment(cop, source, &and_node.right(), diagnostics);
    }
    if let Some(or_node) = node.as_or_node() {
        check_node_for_literal_assignment(cop, source, &or_node.left(), diagnostics);
        check_node_for_literal_assignment(cop, source, &or_node.right(), diagnostics);
    }
}

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_regular_expression_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        LiteralAssignmentInCondition,
        "cops/lint/literal_assignment_in_condition"
    );
}
