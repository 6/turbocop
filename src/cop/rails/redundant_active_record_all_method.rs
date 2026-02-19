use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_ARGUMENT_NODE, CALL_NODE};

pub struct RedundantActiveRecordAllMethod;

const REDUNDANT_AFTER_ALL: &[&[u8]] = &[
    b"where", b"order", b"select", b"find", b"find_by",
    b"first", b"last", b"count", b"pluck", b"sum",
    b"maximum", b"minimum", b"average", b"exists?",
    b"any?", b"none?", b"empty?",
];

/// Methods that could be Enumerable block methods instead of AR query methods.
/// When called with a block, these should NOT be flagged as redundant `all`.
const POSSIBLE_ENUMERABLE_BLOCK_METHODS: &[&[u8]] = &[
    b"any?", b"count", b"find", b"none?", b"one?", b"select", b"sum",
];

impl Cop for RedundantActiveRecordAllMethod {
    fn name(&self) -> &'static str {
        "Rails/RedundantActiveRecordAllMethod"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_ARGUMENT_NODE, CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    ) {
        let allowed_receivers = config.get_string_array("AllowedReceivers");

        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return,
        };

        if chain.inner_method != b"all" {
            return;
        }

        if !REDUNDANT_AFTER_ALL.contains(&chain.outer_method) {
            return;
        }

        // Skip when a possible Enumerable block method is called with a block
        // (e.g., `all.select { |r| r.active? }` uses Ruby's Enumerable#select)
        if POSSIBLE_ENUMERABLE_BLOCK_METHODS.contains(&chain.outer_method) {
            let outer_call = match node.as_call_node() {
                Some(c) => c,
                None => return,
            };
            if outer_call.block().is_some() {
                return;
            }
            // Also check for block pass: all.select(&:active?)
            if let Some(args) = outer_call.arguments() {
                if args.arguments().iter().any(|a| a.as_block_argument_node().is_some()) {
                    return;
                }
            }
        }

        // Skip if receiver of the `all` call is in AllowedReceivers
        if let Some(ref receivers) = allowed_receivers {
            if let Some(recv) = chain.inner_call.receiver() {
                let recv_str = std::str::from_utf8(recv.location().as_slice()).unwrap_or("");
                if receivers.iter().any(|r| r == recv_str) {
                    return;
                }
            }
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Redundant `all` detected. Remove `all` from the chain.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantActiveRecordAllMethod, "cops/rails/redundant_active_record_all_method");
}
