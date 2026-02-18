use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FindByOrAssignmentMemoization;

/// Check if a node is a `find_by` call (not `find_by!`).
fn is_find_by_call(node: &ruby_prism::Node<'_>) -> bool {
    if let Some(call) = node.as_call_node() {
        return call.name().as_slice() == b"find_by";
    }
    false
}

impl Cop for FindByOrAssignmentMemoization {
    fn name(&self) -> &'static str {
        "Rails/FindByOrAssignmentMemoization"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Looking for `@ivar ||= SomeModel.find_by(...)`
        // Prism represents `||=` as InstanceVariableOrWriteNode
        let or_write = match node.as_instance_variable_or_write_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let value = or_write.value();

        // The value should be a direct find_by call (not part of || or ternary)
        if !is_find_by_call(&value) {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid memoizing `find_by` results with `||=`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        FindByOrAssignmentMemoization,
        "cops/rails/find_by_or_assignment_memoization"
    );
}
