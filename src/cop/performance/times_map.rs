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
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.inner_method != b"times"
            || (chain.outer_method != b"map" && chain.outer_method != b"collect")
        {
            return;
        }

        // `times` must have a receiver (e.g. `n.times.map`). Bare `times.map`
        // means `times` is a local variable or method, not Integer#times.
        if chain.inner_call.receiver().is_none() {
            return;
        }

        // Integer#times takes no arguments. Other classes (e.g. Fabricate, Factory)
        // define .times(n, factory) which should not be flagged.
        if chain.inner_call.arguments().is_some() {
            return;
        }

        // RuboCop only flags `times.map` when map/collect has a block (either
        // `{ }` / `do..end` or a block_pass like `&method(:foo)`). Without a
        // block, `times.map` returns an Enumerator and is not an offense.
        let outer_call = node.as_call_node().unwrap();
        if outer_call.block().is_none() {
            return;
        }

        let outer_name = std::str::from_utf8(chain.outer_method).unwrap_or("map");
        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `Array.new` with a block instead of `times.{outer_name}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(TimesMap, "cops/performance/times_map");
}
