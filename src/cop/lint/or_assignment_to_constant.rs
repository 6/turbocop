use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CONSTANT_OR_WRITE_NODE, CONSTANT_PATH_OR_WRITE_NODE};

pub struct OrAssignmentToConstant;

impl Cop for OrAssignmentToConstant {
    fn name(&self) -> &'static str {
        "Lint/OrAssignmentToConstant"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_OR_WRITE_NODE, CONSTANT_PATH_OR_WRITE_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        // ConstantOrWriteNode represents CONST ||= value
        if let Some(n) = node.as_constant_or_write_node() {
            let loc = n.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not use `||=` for assigning to constants.".to_string(),
            ));
        }

        // ConstantPathOrWriteNode represents Foo::BAR ||= value
        if let Some(n) = node.as_constant_path_or_write_node() {
            let loc = n.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not use `||=` for assigning to constants.".to_string(),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OrAssignmentToConstant, "cops/lint/or_assignment_to_constant");
}
