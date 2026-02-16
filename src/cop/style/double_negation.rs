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
        let _enforced_style = config.get_str("EnforcedStyle", "allowed_in_returns");

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

        if let Some(inner_call) = receiver.as_call_node() {
            if inner_call.name().as_slice() == b"!" {
                // Verify inner is also `!` not `not`
                if let Some(msg_loc) = inner_call.message_loc() {
                    if msg_loc.as_slice() == b"not" {
                        return Vec::new();
                    }
                }

                let loc = call_node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid using double negation (`!!`).".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DoubleNegation, "cops/style/double_negation");
}
