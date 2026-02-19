use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, DEF_NODE};

pub struct DefaultScope;

impl Cop for DefaultScope {
    fn name(&self) -> &'static str {
        "Rails/DefaultScope"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, DEF_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Pattern 1: method call `default_scope -> { ... }`
        if let Some(call) = node.as_call_node() {
            if call.receiver().is_none() && call.name().as_slice() == b"default_scope" {
                let loc = call.message_loc().unwrap_or(call.location());
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid use of `default_scope`. It is better to use explicitly named scopes.".to_string(),
                )];
            }
        }

        // Pattern 2: class method `def self.default_scope`
        if let Some(def_node) = node.as_def_node() {
            if def_node.name().as_slice() == b"default_scope" && def_node.receiver().is_some() {
                let loc = def_node.name_loc();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid use of `default_scope`. It is better to use explicitly named scopes.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DefaultScope, "cops/rails/default_scope");
}
