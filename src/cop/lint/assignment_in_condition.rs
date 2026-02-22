use crate::cop::node_type::{
    CLASS_VARIABLE_WRITE_NODE, CONSTANT_WRITE_NODE, GLOBAL_VARIABLE_WRITE_NODE, IF_NODE,
    INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE, UNLESS_NODE, UNTIL_NODE, WHILE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AssignmentInCondition;

impl Cop for AssignmentInCondition {
    fn name(&self) -> &'static str {
        "Lint/AssignmentInCondition"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CLASS_VARIABLE_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            IF_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let allow_safe = config.get_bool("AllowSafeAssignment", true);

        let predicate = if let Some(if_node) = node.as_if_node() {
            Some(if_node.predicate())
        } else if let Some(while_node) = node.as_while_node() {
            Some(while_node.predicate())
        } else if let Some(until_node) = node.as_until_node() {
            Some(until_node.predicate())
        } else if let Some(unless_node) = node.as_unless_node() {
            Some(unless_node.predicate())
        } else {
            None
        };

        let predicate = match predicate {
            Some(p) => p,
            None => return,
        };

        // Check if the predicate is an assignment
        let is_assignment = predicate.as_local_variable_write_node().is_some()
            || predicate.as_instance_variable_write_node().is_some()
            || predicate.as_class_variable_write_node().is_some()
            || predicate.as_global_variable_write_node().is_some()
            || predicate.as_constant_write_node().is_some();

        if !is_assignment {
            return;
        }

        // AllowSafeAssignment: if the assignment is wrapped in parens, allow it
        if allow_safe {
            // If the predicate is a parenthesized expression, it's "safe"
            // In Prism, the if_node.predicate() already unwraps parens,
            // so we need to check the raw source for parens around the assignment.
            // Actually, Prism keeps ParenthesesNode wrapping. But the predicate
            // is already the inner node. Let's check by looking at the source text.
            // A simpler approach: check if there's a `(` immediately before the assignment.
            let pred_loc = predicate.location();
            let start = pred_loc.start_offset();
            if start > 0 {
                let bytes = source.as_bytes();
                // Walk backwards skipping whitespace to find a `(`
                let mut pos = start - 1;
                while pos > 0 && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                    pos -= 1;
                }
                if bytes[pos] == b'(' {
                    return; // Safe assignment: if (x = 1)
                }
            }
        }

        let loc = predicate.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Assignment in condition detected. Did you mean `==`?".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AssignmentInCondition, "cops/lint/assignment_in_condition");
}
