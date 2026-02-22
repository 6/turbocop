use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SafeNavigationChainLength;

impl Cop for SafeNavigationChainLength {
    fn name(&self) -> &'static str {
        "Style/SafeNavigationChainLength"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let max = config.get_usize("Max", 2);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must use safe navigation (&.)
        if !is_safe_nav(&call) {
            return;
        }

        // Count the chain length
        let chain_len = count_safe_nav_chain(node);
        if chain_len <= max {
            return;
        }

        // Only report on the outermost call in the chain
        // (skip if this node is itself a receiver of another &. call)
        // We can't walk up, so we report on every call that exceeds the limit
        // but only the outermost will have the full chain.

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Do not chain more than {} safe navigation operators. (found {})",
                max, chain_len
            ),
        ));
    }
}

fn is_safe_nav(call: &ruby_prism::CallNode<'_>) -> bool {
    if let Some(op) = call.call_operator_loc() {
        op.as_slice() == b"&."
    } else {
        false
    }
}

fn count_safe_nav_chain(node: &ruby_prism::Node<'_>) -> usize {
    let call = match node.as_call_node() {
        Some(c) => c,
        None => return 0,
    };

    if !is_safe_nav(&call) {
        return 0;
    }

    let recv_count = match call.receiver() {
        Some(r) => count_safe_nav_chain(&r),
        None => 0,
    };

    1 + recv_count
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SafeNavigationChainLength,
        "cops/style/safe_navigation_chain_length"
    );
}
