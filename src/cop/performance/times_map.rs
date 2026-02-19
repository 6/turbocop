use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct TimesMap;

impl Cop for TimesMap {
    fn name(&self) -> &'static str {
        "Performance/TimesMap"
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

        if chain.inner_method != b"times" || (chain.outer_method != b"map" && chain.outer_method != b"collect") {
            return;
        }

        let outer_name = std::str::from_utf8(chain.outer_method).unwrap_or("map");
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, format!("Use `Array.new` with a block instead of `times.{outer_name}`.")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(TimesMap, "cops/performance/times_map");
}
