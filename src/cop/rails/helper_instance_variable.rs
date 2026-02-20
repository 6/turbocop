use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{INSTANCE_VARIABLE_READ_NODE, INSTANCE_VARIABLE_WRITE_NODE};

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

    fn interested_node_types(&self) -> &'static [u8] {
        &[INSTANCE_VARIABLE_READ_NODE, INSTANCE_VARIABLE_WRITE_NODE]
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
        let loc;

        if node.as_instance_variable_read_node().is_some() {
            loc = node.location();
        } else if node.as_instance_variable_write_node().is_some() {
            loc = node.location();
        } else {
            return;
        }

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not use instance variables in helpers.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(HelperInstanceVariable, "cops/rails/helper_instance_variable");
}
