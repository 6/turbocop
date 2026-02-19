use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct SelectMap;

impl Cop for SelectMap {
    fn name(&self) -> &'static str {
        "Performance/SelectMap"
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
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.outer_method != b"map" {
            return;
        }

        let inner = chain.inner_method;
        let inner_name = if inner == b"select" {
            "select"
        } else if inner == b"filter" {
            "filter"
        } else {
            return;
        };

        // The inner call should have a block
        if chain.inner_call.block().is_none() {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, format!("Use `filter_map` instead of `{inner_name}...map`.")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SelectMap, "cops/performance/select_map");
}
