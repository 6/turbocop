use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MapCompact;

impl Cop for MapCompact {
    fn name(&self) -> &'static str {
        "Performance/MapCompact"
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

        if chain.outer_method != b"compact" {
            return;
        }

        let inner = chain.inner_method;
        if inner != b"map" && inner != b"collect" {
            return;
        }

        // The inner call should have a block (either { } / do..end or &:symbol)
        if chain.inner_call.block().is_none() {
            return;
        }

        // Report at the inner method selector (map/collect), matching RuboCop
        let msg_loc = match chain.inner_call.message_loc() {
            Some(loc) => loc,
            None => return,
        };
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `filter_map` instead.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapCompact, "cops/performance/map_compact");
}
