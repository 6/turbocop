use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-18):
///
/// **FP root cause (5 FPs):** RuboCop v1.84.2 with TargetRubyVersion >= 3.4 uses
/// parser_prism, which produces `itblock` AST nodes for blocks using the `it` keyword
/// (Ruby 3.4+) and `numblock` nodes for numbered parameters (`_1`). The cop's
/// NodePattern `(block ...)` does not match `itblock` or `numblock`, so RuboCop
/// does not flag these patterns. Prism (used by nitrocop) always uses `BlockNode`
/// with `ItParametersNode`/`NumberedParametersNode` as parameters, so nitrocop was
/// incorrectly flagging them. Fixed by checking the block's parameters and skipping
/// `ItParametersNode` and `NumberedParametersNode`.
///
/// **FN root cause (9 FNs):** All FNs were `select { ... }.present?` — RuboCop flags
/// `present?` (→ `any?`) and `many?` (→ `many?`) when `ActiveSupportExtensionsEnabled`
/// is true (set by rubocop-rails). nitrocop was missing these methods entirely. Fixed
/// by adding `present?` and `many?` handling gated on the config setting.
pub struct RedundantFilterChain;

const FILTER_METHODS: &[&[u8]] = &[b"select", b"filter", b"find_all"];

impl Cop for RedundantFilterChain {
    fn name(&self) -> &'static str {
        "Style/RedundantFilterChain"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_bytes = call.name().as_slice();

        let active_support = config.get_bool("ActiveSupportExtensionsEnabled", false);

        // Must be any?, empty?, none?, one?, or (with ActiveSupport) present?, many?
        let replacement = match method_bytes {
            b"any?" => "any?",
            b"empty?" => "none?",
            b"none?" => "none?",
            b"one?" => "one?",
            b"present?" if active_support => "any?",
            b"many?" if active_support => "many?",
            _ => return,
        };

        // Must have no arguments or block
        if call.arguments().is_some() || call.block().is_some() {
            return;
        }

        // Receiver must be a filter method with a block
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let recv_method = recv_call.name();
        let recv_bytes = recv_method.as_slice();

        if !FILTER_METHODS.contains(&recv_bytes) {
            return;
        }

        // The filter method must have a block (or block pass)
        let block = match recv_call.block() {
            Some(b) => b,
            None => return,
        };

        // Skip blocks using `it` keyword (ItParametersNode) or numbered parameters
        // (NumberedParametersNode). RuboCop's parser_prism produces `itblock`/`numblock`
        // AST nodes for these, which don't match the cop's `(block ...)` NodePattern.
        if let Some(block_node) = block.as_block_node() {
            if let Some(params) = block_node.parameters() {
                if params.as_it_parameters_node().is_some()
                    || params.as_numbered_parameters_node().is_some()
                {
                    return;
                }
            }
        }

        let filter_str = std::str::from_utf8(recv_bytes).unwrap_or("select");
        let predicate_str = std::str::from_utf8(method_bytes).unwrap_or("any?");

        let msg_loc = recv_call
            .message_loc()
            .unwrap_or_else(|| recv_call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{replacement}` instead of `{filter_str}.{predicate_str}`."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantFilterChain, "cops/style/redundant_filter_chain");

    #[test]
    fn present_with_active_support() {
        use crate::testutil::assert_cop_offenses_full_with_config;
        let source = b"arr.select { |x| x > 1 }.present?\n    ^^^^^^ Style/RedundantFilterChain: Use `any?` instead of `select.present?`.\n";
        let mut config = CopConfig::default();
        config
            .options
            .insert("ActiveSupportExtensionsEnabled".to_string(), true.into());
        assert_cop_offenses_full_with_config(&RedundantFilterChain, source, config);
    }

    #[test]
    fn present_without_active_support() {
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        let source = b"arr.select { |x| x > 1 }.present?\n";
        let config = CopConfig::default();
        assert_cop_no_offenses_full_with_config(&RedundantFilterChain, source, config);
    }

    #[test]
    fn many_with_active_support() {
        use crate::testutil::assert_cop_offenses_full_with_config;
        let source = b"arr.filter { |x| x > 1 }.many?\n    ^^^^^^ Style/RedundantFilterChain: Use `many?` instead of `filter.many?`.\n";
        let mut config = CopConfig::default();
        config
            .options
            .insert("ActiveSupportExtensionsEnabled".to_string(), true.into());
        assert_cop_offenses_full_with_config(&RedundantFilterChain, source, config);
    }

    #[test]
    fn many_without_active_support() {
        use crate::testutil::assert_cop_no_offenses_full_with_config;
        let source = b"arr.filter { |x| x > 1 }.many?\n";
        let config = CopConfig::default();
        assert_cop_no_offenses_full_with_config(&RedundantFilterChain, source, config);
    }
}
