use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct OrAssignmentToConstant;

impl Cop for OrAssignmentToConstant {
    fn name(&self) -> &'static str {
        "Lint/OrAssignmentToConstant"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // ConstantOrWriteNode represents CONST ||= value
        if let Some(n) = node.as_constant_or_write_node() {
            let loc = n.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Do not use `||=` for assigning to constants.".to_string(),
            )];
        }

        // ConstantPathOrWriteNode represents Foo::BAR ||= value
        if let Some(n) = node.as_constant_path_or_write_node() {
            let loc = n.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Do not use `||=` for assigning to constants.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OrAssignmentToConstant, "cops/lint/or_assignment_to_constant");
}
