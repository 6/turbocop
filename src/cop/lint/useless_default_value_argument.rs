use crate::cop::shared::node_type::{CALL_NODE, KEYWORD_HASH_NODE};
use crate::cop::shared::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Checks for `fetch` or `Array.new` with both a default value argument and a block.
/// The block always supersedes the default value argument.
///
/// ## Corpus investigation (2026-03-07)
/// 17 FPs, 0 FNs. All FPs were `fetch` calls with a forwarded `&block` argument
/// (e.g., `@cache.fetch(key, options, &block)`). In Prism, `call.block()` returns
/// `Some(BlockArgumentNode)` for `&block` forwarding, not just `Some(BlockNode)` for
/// literal blocks. RuboCop's NodePattern uses `any_block` which only matches literal
/// blocks (`block`/`numblock`), so `&block` forwarding is never flagged. Fixed by
/// checking that the block is a `BlockNode`, not a `BlockArgumentNode`.
pub struct UselessDefaultValueArgument;

impl Cop for UselessDefaultValueArgument {
    fn name(&self) -> &'static str {
        "Lint/UselessDefaultValueArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, KEYWORD_HASH_NODE]
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

        // Must have a literal block (not a forwarded &block argument).
        // In Prism, call.block() returns BlockArgumentNode for &block forwarding,
        // but only literal blocks ({ } / do..end) should trigger the cop.
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };
        if block.as_block_argument_node().is_some() {
            return;
        }

        let method_name = call.name().as_slice();

        if method_name == b"fetch" {
            // Must have a receiver (not a bare fetch call)
            let receiver = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            // Skip if receiver is in AllowedReceivers
            let allowed_receivers = config
                .get_string_array("AllowedReceivers")
                .unwrap_or_default();
            if !allowed_receivers.is_empty() {
                let recv_bytes = receiver.location().as_slice();
                let recv_str = std::str::from_utf8(recv_bytes).unwrap_or("");
                if allowed_receivers.iter().any(|r| r == recv_str) {
                    return;
                }
            }

            // Must have 2 arguments (key and default_value)
            let args = match call.arguments() {
                Some(a) => a,
                None => return,
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 2 {
                return;
            }

            // Skip if second argument is a keyword hash (keyword arguments)
            if arg_list[1].as_keyword_hash_node().is_some() {
                return;
            }

            let default_loc = arg_list[1].location();
            let (line, column) = source.offset_to_line_col(default_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Block supersedes default value argument.".to_string(),
            ));
        } else if method_name == b"new" {
            // Check for Array.new(size, default) { block }
            let receiver = match call.receiver() {
                Some(r) => r,
                None => return,
            };

            let recv_name = match constant_name(&receiver) {
                Some(n) => n,
                None => return,
            };

            if recv_name != b"Array" {
                return;
            }

            let args = match call.arguments() {
                Some(a) => a,
                None => return,
            };
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if arg_list.len() != 2 {
                return;
            }

            let default_loc = arg_list[1].location();
            let (line, column) = source.offset_to_line_col(default_loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Block supersedes default value argument.".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        UselessDefaultValueArgument,
        "cops/lint/useless_default_value_argument"
    );
}
