use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DoubleStartEndWith;

impl Cop for DoubleStartEndWith {
    fn name(&self) -> &'static str {
        "Performance/DoubleStartEndWith"
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
        let or_node = match node.as_or_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let left_call = match or_node.left().as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let right_call = match or_node.right().as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let left_name = left_call.name().as_slice();
        let right_name = right_call.name().as_slice();

        // Both sides must use the same method: start_with? or end_with?
        if left_name != right_name {
            return Vec::new();
        }

        if left_name != b"start_with?" && left_name != b"end_with?" {
            return Vec::new();
        }

        let method_display = if left_name == b"start_with?" {
            "start_with?"
        } else {
            "end_with?"
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, format!(
            "Use `{method_display}` with multiple arguments instead of chaining `||`."
        ))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DoubleStartEndWith, "cops/performance/double_start_end_with");
}
