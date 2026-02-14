use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantActiveRecordAllMethod;

const REDUNDANT_AFTER_ALL: &[&[u8]] = &[
    b"where", b"order", b"select", b"find", b"find_by",
    b"first", b"last", b"count", b"pluck", b"sum",
    b"maximum", b"minimum", b"average", b"exists?",
    b"any?", b"none?", b"empty?",
];

impl Cop for RedundantActiveRecordAllMethod {
    fn name(&self) -> &'static str {
        "Rails/RedundantActiveRecordAllMethod"
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

        if chain.inner_method != b"all" {
            return Vec::new();
        }

        if !REDUNDANT_AFTER_ALL.contains(&chain.outer_method) {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Redundant `all` detected. Remove `all` from the chain.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantActiveRecordAllMethod, "cops/rails/redundant_active_record_all_method");
}
