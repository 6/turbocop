use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Attr;

impl Cop for Attr {
    fn name(&self) -> &'static str {
        "Style/Attr"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be a bare `attr` call (no receiver)
        if call_node.name().as_slice() != b"attr" {
            return Vec::new();
        }
        if call_node.receiver().is_some() {
            return Vec::new();
        }

        // Must have arguments
        if call_node.arguments().is_none() {
            return Vec::new();
        }

        let msg_loc = call_node.message_loc().unwrap_or_else(|| call_node.location());
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not use `attr`. Use `attr_reader` instead.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Attr, "cops/style/attr");
}
