use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct FlatMap;

impl Cop for FlatMap {
    fn name(&self) -> &'static str {
        "Performance/FlatMap"
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

        if chain.outer_method != b"flatten" {
            return Vec::new();
        }

        let inner = chain.inner_method;
        let inner_name = if inner == b"map" {
            "map"
        } else if inner == b"collect" {
            "collect"
        } else {
            return Vec::new();
        };

        // The inner call should have a block
        if chain.inner_call.block().is_none() {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, format!("Use `flat_map` instead of `{inner_name}...flatten`."))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FlatMap, "cops/performance/flat_map");
}
