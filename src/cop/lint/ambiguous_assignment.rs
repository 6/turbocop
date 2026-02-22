use crate::cop::node_type::{
    CLASS_VARIABLE_WRITE_NODE, CONSTANT_PATH_WRITE_NODE, CONSTANT_WRITE_NODE,
    GLOBAL_VARIABLE_WRITE_NODE, INSTANCE_VARIABLE_WRITE_NODE, LOCAL_VARIABLE_WRITE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AmbiguousAssignment;

const MISTAKES: &[(&[u8], &str)] = &[(b"=-", "-="), (b"=+", "+="), (b"=*", "*="), (b"=!", "!=")];

impl Cop for AmbiguousAssignment {
    fn name(&self) -> &'static str {
        "Lint/AmbiguousAssignment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CLASS_VARIABLE_WRITE_NODE,
            CONSTANT_PATH_WRITE_NODE,
            CONSTANT_WRITE_NODE,
            GLOBAL_VARIABLE_WRITE_NODE,
            INSTANCE_VARIABLE_WRITE_NODE,
            LOCAL_VARIABLE_WRITE_NODE,
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
        // Check variable assignment node types
        let (operator_loc, value) = if let Some(n) = node.as_local_variable_write_node() {
            (n.operator_loc(), n.value())
        } else if let Some(n) = node.as_instance_variable_write_node() {
            (n.operator_loc(), n.value())
        } else if let Some(n) = node.as_class_variable_write_node() {
            (n.operator_loc(), n.value())
        } else if let Some(n) = node.as_global_variable_write_node() {
            (n.operator_loc(), n.value())
        } else if let Some(n) = node.as_constant_write_node() {
            (n.operator_loc(), n.value())
        } else if let Some(n) = node.as_constant_path_write_node() {
            (n.operator_loc(), n.value().into())
        } else {
            return;
        };

        // RuboCop takes: range from (operator.end_pos - 1) to (rhs.begin_pos + 1)
        // This captures the `=` and the first char of the RHS.
        // For `x =+ y`: operator is `=`, rhs starts at `+`, so the range captures `=+`.
        // For `x = +y`: operator is `=`, rhs starts at `+`, range captures `= ` (space) plus `+`
        //   but actually `= +` is 3 chars from end_pos-1 to begin_pos+1.
        // Wait, let me reconsider: end_pos - 1 = position of `=`, begin_pos + 1 = position after first char of rhs.
        // For `x =+ y`: `=` at col 2, end_pos=3, end_pos-1=2. `+` at col 3, begin_pos=3, begin_pos+1=4.
        //   range_source = source[2..4] = "=+" -> matches MISTAKES
        // For `x = + y`: `=` at col 2, end_pos=3, end_pos-1=2. `+` at col 4, begin_pos=4, begin_pos+1=5.
        //   range_source = source[2..5] = "= +" -> doesn't match (3 chars)

        let src = source.as_bytes();
        let eq_end = operator_loc.start_offset() + operator_loc.as_slice().len();
        let value_start = value.location().start_offset();

        if eq_end == 0 || value_start >= src.len() {
            return;
        }

        let range_start = eq_end - 1; // Last char of operator (the `=`)
        let range_end = value_start + 1; // First char of rhs + 1

        if range_end > src.len() || range_start >= range_end {
            return;
        }

        let range_source = &src[range_start..range_end];

        for &(mistake, correction) in MISTAKES {
            if range_source == mistake {
                let (line, column) = source.offset_to_line_col(range_start);
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    format!("Suspicious assignment detected. Did you mean `{correction}`?"),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AmbiguousAssignment, "cops/lint/ambiguous_assignment");
}
