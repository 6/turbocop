use crate::cop::shared::node_type::{CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::shared::util::RSPEC_DEFAULT_INCLUDE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-14)
///
/// Corpus oracle reported FP=1, FN=9.
///
/// FP=1: No example locations available (older corpus run without full example
/// storage). Cannot diagnose without specific file/line context. The cop
/// checks for `describe`/`context`/`feature` method name patterns — possible
/// FP causes: a non-RSpec library using these method names with a string
/// description, or a receiver-qualified call. No code fix attempted without
/// concrete reproduction.
///
/// ## Corpus investigation (2026-03-15)
///
/// FN=10: The cop only checked `as_string_node()` for the first argument, missing
/// `InterpolatedStringNode`. When the context description uses string interpolation
/// (e.g., `context "#to_boolean for #{value.inspect}" do`), Prism parses it as an
/// `InterpolatedStringNode`. Fix: also check `as_interpolated_string_node()` and
/// extract the leading text from the first `StringNode` part to determine if it
/// starts with `#` or `.`.
///
/// ## Corpus investigation (2026-03-18)
///
/// FP=1 at drhenner__ror_ecommerce spec/models/order_spec.rb:235. The call
/// `context ".create_invoice_transaction(...)"` has no block (no `do...end`).
/// RuboCop uses `on_block` handler which only fires for block nodes, so blockless
/// `context` calls are never checked. Also added receiver check (RuboCop uses
/// `send nil? :context`). Fix: guard on `call.block()` being a `BlockNode` and
/// `call.receiver().is_none()`.
pub struct ContextMethod;

impl Cop for ContextMethod {
    fn name(&self) -> &'static str {
        "RSpec/ContextMethod"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_DEFAULT_INCLUDE
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTERPOLATED_STRING_NODE, STRING_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"context" {
            return;
        }

        // RuboCop uses on_block handler — only fires when call has a block
        if call.block().is_none_or(|b| b.as_block_node().is_none()) {
            return;
        }

        // Receiver must be nil (RuboCop: `send nil? :context`)
        if call.receiver().is_some() {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<ruby_prism::Node<'_>> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Extract description text from StringNode or InterpolatedStringNode
        let content_str: String;
        if let Some(s) = arg_list[0].as_string_node() {
            let content = s.unescaped();
            content_str = match std::str::from_utf8(content) {
                Ok(s) => s.to_string(),
                Err(_) => return,
            };
        } else if let Some(interp) = arg_list[0].as_interpolated_string_node() {
            // For interpolated strings, extract leading text before first interpolation.
            let parts: Vec<_> = interp.parts().iter().collect();
            content_str = if let Some(first) = parts.first() {
                if let Some(s) = first.as_string_node() {
                    let text = s.unescaped();
                    std::str::from_utf8(text)
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                } else {
                    return;
                }
            } else {
                return;
            };
        } else {
            return;
        };

        // Flag if starts with '#' or '.'
        if !content_str.starts_with('#') && !content_str.starts_with('.') {
            return;
        }

        let loc = arg_list[0].location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `describe` for testing methods.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ContextMethod, "cops/rspec/context_method");
}
