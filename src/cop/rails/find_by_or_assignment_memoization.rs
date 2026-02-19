use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE};

pub struct FindByOrAssignmentMemoization;

fn trim_ascii_start(s: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < s.len() && s[i].is_ascii_whitespace() {
        i += 1;
    }
    &s[i..]
}

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

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INSTANCE_VARIABLE_OR_WRITE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Looking for `@ivar ||= SomeModel.find_by(...)`
        // Prism represents `||=` as InstanceVariableOrWriteNode
        let or_write = match node.as_instance_variable_or_write_node() {
            Some(n) => n,
            None => return,
        };

        let value = or_write.value();

        // The value should be a direct find_by call (not part of || or ternary)
        if !is_find_by_call(&value) {
            return;
        }

        // RuboCop skips when the ||= is inside an if/unless (including modifiers).
        // Since we don't have ancestor tracking, check if the IfNode wrapping this
        // node extends beyond our range (indicating a modifier if/unless).
        // Practically: check if the node is wrapped in an IfNode by looking at the
        // source bytes just after the find_by call's closing paren for ` if ` or ` unless `.
        let node_end = node.location().end_offset();
        let src = source.as_bytes();
        if node_end < src.len() {
            let after = &src[node_end..];
            // Look for modifier if/unless on the same line
            let line_rest: &[u8] = after.split(|&b| b == b'\n').next().unwrap_or(after);
            let trimmed = trim_ascii_start(line_rest);
            if trimmed.starts_with(b"if ")
                || trimmed.starts_with(b"unless ")
                || trimmed.starts_with(b"if\t")
                || trimmed.starts_with(b"unless\t")
            {
                return;
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid memoizing `find_by` results with `||=`.".to_string(),
        ));
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
