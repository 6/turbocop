use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::IT_LOCAL_VARIABLE_READ_NODE;

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

    fn interested_node_types(&self) -> &'static [u8] {
        &[IT_LOCAL_VARIABLE_READ_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // RuboCop: maximum_target_ruby_version 3.3
        // In Ruby 3.4+, `it` is the official anonymous block parameter, so this
        // warning is no longer relevant.
        let ruby_version = config
            .options
            .get("TargetRubyVersion")
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(2.7);
        if ruby_version >= 3.4 {
            return Vec::new();
        }

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
