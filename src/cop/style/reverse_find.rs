use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ReverseFind;

impl Cop for ReverseFind {
    fn name(&self) -> &'static str {
        "Style/ReverseFind"
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

        // Must be `.find`
        if call.name().as_slice() != b"find" {
            return Vec::new();
        }

        // Receiver must be a `.reverse` call
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let recv_call = match receiver.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if recv_call.name().as_slice() != b"reverse" {
            return Vec::new();
        }

        // `.reverse` must have no arguments
        if recv_call.arguments().is_some() {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `rfind` instead of `reverse.find`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReverseFind, "cops/style/reverse_find");
}
