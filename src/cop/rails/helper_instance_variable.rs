use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let loc;

        if node.as_instance_variable_read_node().is_some() {
            loc = node.location();
        } else if node.as_instance_variable_write_node().is_some() {
            loc = node.location();
        } else {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not use instance variables in helpers.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HelperInstanceVariable, "cops/rails/helper_instance_variable");
}
