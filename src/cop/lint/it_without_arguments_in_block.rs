use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for `it` calls without arguments inside blocks without parameters.
/// In Ruby 3.4, `it` refers to the first block parameter, so this warns about
/// ambiguous usage in Ruby < 3.4.
pub struct ItWithoutArgumentsInBlock;

impl Cop for ItWithoutArgumentsInBlock {
    fn name(&self) -> &'static str {
        "Lint/ItWithoutArgumentsInBlock"
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
        // In Prism 1.9+, bare `it` in a parameterless block is parsed as
        // ItLocalVariableReadNode, not as a CallNode.
        if let Some(it_node) = node.as_it_local_variable_read_node() {
            let loc = it_node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "`it` calls without arguments will refer to the first block param in Ruby 3.4; use `it()` or `self.it`.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ItWithoutArgumentsInBlock, "cops/lint/it_without_arguments_in_block");
}
