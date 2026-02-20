use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE, SYMBOL_NODE};

pub struct MapMethodChain;

/// Check if a call node has a block_pass argument with a symbol (e.g., `&:foo`).
fn has_symbol_block_pass(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(block) = call.block() {
        if let Some(bp) = block.as_block_argument_node() {
            if let Some(expr) = bp.expression() {
                return expr.as_symbol_node().is_some();
            }
        }
    }
    false
}

impl Cop for MapMethodChain {
    fn name(&self) -> &'static str {
        "Performance/MapMethodChain"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE, SYMBOL_NODE]
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
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let outer_method = outer_call.name().as_slice();
        if outer_method != b"map" && outer_method != b"collect" {
            return;
        }

        // Outer call must have a block_pass with symbol arg (e.g., map(&:foo))
        if !has_symbol_block_pass(&outer_call) {
            return;
        }

        // Inner call (receiver) must also be map/collect with symbol block_pass
        let inner_node = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };
        let inner_call = match inner_node.as_call_node() {
            Some(c) => c,
            None => return,
        };
        let inner_method = inner_call.name().as_slice();
        if inner_method != b"map" && inner_method != b"collect" {
            return;
        }
        if !has_symbol_block_pass(&inner_call) {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, "Use `map` with a block instead of chaining multiple `map` calls with symbol arguments.".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MapMethodChain, "cops/performance/map_method_chain");
}
