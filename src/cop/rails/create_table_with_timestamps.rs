use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct CreateTableWithTimestamps;

/// Walk a node tree looking for a call to `timestamps`.
struct TimestampFinder {
    found: bool,
}

impl<'pr> Visit<'pr> for TimestampFinder {
    fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"timestamps" {
                self.found = true;
            }
        }
    }

    fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
        if let Some(call) = node.as_call_node() {
            if call.name().as_slice() == b"timestamps" {
                self.found = true;
            }
        }
    }
}

impl Cop for CreateTableWithTimestamps {
    fn name(&self) -> &'static str {
        "Rails/CreateTableWithTimestamps"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &["db/migrate/**/*.rb"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Start from CallNode `create_table`, then access its block
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"create_table" {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Walk block body looking for timestamps call
        let body = match block_node.body() {
            Some(b) => b,
            None => {
                // Empty block -- flag it
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Add `t.timestamps` to `create_table` block.".to_string(),
                )];
            }
        };

        let mut finder = TimestampFinder { found: false };
        finder.visit(&body);

        if finder.found {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Add `t.timestamps` to `create_table` block.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CreateTableWithTimestamps, "cops/rails/create_table_with_timestamps");
}
