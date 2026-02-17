use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Send;

impl Cop for Send {
    fn name(&self) -> &'static str {
        "Style/Send"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be `send` method
        if call.name().as_slice() != b"send" {
            return Vec::new();
        }

        // Must have arguments
        if call.arguments().is_none() {
            return Vec::new();
        }

        // Must have a receiver (Foo.send, not bare send)
        if call.receiver().is_none() {
            return Vec::new();
        }

        let msg_loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Prefer `Object#__send__` or `Object#public_send` to `send`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Send, "cops/style/send");
}
