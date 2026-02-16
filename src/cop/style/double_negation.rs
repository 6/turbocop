use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DoubleNegation;

impl Cop for DoubleNegation {
    fn name(&self) -> &'static str {
        "Style/DoubleNegation"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let enforced_style = config.get_str("EnforcedStyle", "allowed_in_returns");

        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be the `!` method
        if call_node.name().as_slice() != b"!" {
            return Vec::new();
        }

        // Check the message_loc to ensure it's `!` not `not`
        if let Some(msg_loc) = call_node.message_loc() {
            if msg_loc.as_slice() == b"not" {
                return Vec::new();
            }
        }

        // Receiver must also be a `!` call
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let inner_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if inner_call.name().as_slice() != b"!" {
            return Vec::new();
        }

        // Verify inner is also `!` not `not`
        if let Some(msg_loc) = inner_call.message_loc() {
            if msg_loc.as_slice() == b"not" {
                return Vec::new();
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
                    return Vec::new();
                }
            }

            // More importantly: check if we're the last expression in a method body.
            // We use a heuristic: if the !! starts on a line and the next non-whitespace
            // after the !! expression is `end` or end of file, it's a return position.
            // This is a simplification, but catches the common case of:
            //   def foo?
            //     !!bar
            //   end
            let end_offset = loc.end_offset();
            if end_offset < src.len() {
                let after = &src[end_offset..];
                // Skip whitespace and newlines, see what comes next
                let trimmed_after = after.trim_ascii_start();
                if trimmed_after.starts_with(b"end")
                    && (trimmed_after.len() == 3
                        || !trimmed_after[3..4].iter().all(|&b: &u8| b.is_ascii_alphanumeric() || b == b'_'))
                {
                    return Vec::new();
                }
            }
        }

        let loc = call_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid the use of double negation (`!!`).".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DoubleNegation, "cops/style/double_negation");
}
