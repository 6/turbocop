use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SelectMap;

impl Cop for SelectMap {
    fn name(&self) -> &'static str {
        "Rails/SelectMap"
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
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        if chain.outer_method != b"map" {
            return Vec::new();
        }

        if chain.inner_method != b"select" {
            return Vec::new();
        }

        // Both should have blocks
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if outer_call.block().is_none() {
            return Vec::new();
        }
        if chain.inner_call.block().is_none() {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `filter_map` instead of `select.map`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SelectMap, "cops/rails/select_map");
}
