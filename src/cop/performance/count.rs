use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Count;

impl Cop for Count {
    fn name(&self) -> &'static str {
        "Performance/Count"
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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        let outer = chain.outer_method;
        let outer_name = if outer == b"count" {
            "count"
        } else if outer == b"size" {
            "size"
        } else if outer == b"length" {
            "length"
        } else {
            return;
        };

        let inner = chain.inner_method;
        let inner_name = if inner == b"select" {
            "select"
        } else if inner == b"reject" {
            "reject"
        } else if inner == b"filter" {
            "filter"
        } else if inner == b"find_all" {
            "find_all"
        } else {
            return;
        };

        // The inner call must have a block (normal block or block_pass like &:symbol)
        if chain.inner_call.block().is_none() {
            return;
        }

        // Skip if the outer call (count/size/length) itself has a block:
        // e.g. `select { |e| e.odd? }.count { |e| e > 2 }` is allowed
        let outer_call = node.as_call_node().unwrap();
        if outer_call.block().is_some() {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `count` instead of `{inner_name}...{outer_name}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Count, "cops/performance/count");
}
