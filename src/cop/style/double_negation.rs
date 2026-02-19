use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct DoubleNegation;

impl Cop for DoubleNegation {
    fn name(&self) -> &'static str {
        "Style/DoubleNegation"
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
    ) {
        let enforced_style = config.get_str("EnforcedStyle", "allowed_in_returns");

        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be the `!` method
        if call_node.name().as_slice() != b"!" {
            return;
        }

        // Check the message_loc to ensure it's `!` not `not`
        if let Some(msg_loc) = call_node.message_loc() {
            if msg_loc.as_slice() == b"not" {
                return;
            }
        }

        // Receiver must also be a `!` call
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"!" {
            return;
        }

        // Verify inner is also `!` not `not`
        if let Some(msg_loc) = inner_call.message_loc() {
            if msg_loc.as_slice() == b"not" {
                return;
            }
        }

        // For "allowed_in_returns" style (default), skip !! when used as a return value.
        // We approximate this by checking if the !! is at a position that looks like
        // a return value: the last expression in the source line context.
        // This is checked by seeing if the !! node is immediately preceded by `return`
        // or if it's a standalone expression (not part of an assignment, hash, or array).
        if enforced_style == "allowed_in_returns" {
            // Check if this is preceded by `return` keyword
            let loc = call_node.location();
            let start = loc.start_offset();
            let src = source.as_bytes();
            // Look backwards for `return` keyword
            if start >= 7 {
                let prefix = &src[..start];
                let trimmed = prefix.trim_ascii_end();
                if trimmed.ends_with(b"return") {
                    return;
                }
            }

            // Check if we're the last expression in a method body.
            // Skip to the end of the current statement (end of line, possibly
            // continuing onto subsequent lines), then check if `end` follows.
            //
            // This handles common patterns:
            //   def foo?
            //     !!bar
            //   end
            // and also:
            //   def comparison?
            //     !!simple_comparison(node) || nested_comparison?(node)
            //   end
            let end_offset = loc.end_offset();
            if end_offset < src.len() {
                // Skip to end of line (the !! might be part of a larger expression)
                let mut pos = end_offset;
                while pos < src.len() && src[pos] != b'\n' {
                    pos += 1;
                }
                // Now skip blank lines and check what comes next
                let after = &src[pos..];
                let trimmed_after = after.trim_ascii_start();
                if trimmed_after.starts_with(b"end")
                    && (trimmed_after.len() == 3
                        || !trimmed_after[3..4].iter().all(|&b: &u8| b.is_ascii_alphanumeric() || b == b'_'))
                {
                    return;
                }
            }
        }

        let loc = call_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid the use of double negation (`!!`).".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DoubleNegation, "cops/style/double_negation");
}
