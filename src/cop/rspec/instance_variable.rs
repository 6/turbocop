use crate::cop::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct InstanceVariable;

impl Cop for InstanceVariable {
    fn name(&self) -> &'static str {
        "RSpec/InstanceVariable"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Detect @ivar reads and writes in spec files
        let loc = if let Some(n) = node.as_instance_variable_read_node() {
            n.location()
        } else if let Some(n) = node.as_instance_variable_write_node() {
            n.location()
        } else if let Some(n) = node.as_instance_variable_operator_write_node() {
            n.location()
        } else if let Some(n) = node.as_instance_variable_or_write_node() {
            n.location()
        } else if let Some(n) = node.as_instance_variable_and_write_node() {
            n.location()
        } else {
            return Vec::new();
        };

        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid instance variables - use let, a method call, or a local variable (if possible)."
                .to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(InstanceVariable, "cops/rspec/instance_variable");
}
