use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ChainArrayAllocation;

const INNER_METHODS: &[&[u8]] = &[b"compact", b"flatten", b"sort", b"uniq"];
const OUTER_METHODS: &[&[u8]] = &[b"map", b"flat_map"];

impl Cop for ChainArrayAllocation {
    fn name(&self) -> &'static str {
        "Performance/ChainArrayAllocation"
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

        if !INNER_METHODS.contains(&chain.inner_method) {
            return;
        }

        if !OUTER_METHODS.contains(&chain.outer_method) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, "Avoid chaining array methods that allocate intermediate arrays.".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ChainArrayAllocation, "cops/performance/chain_array_allocation");
}
