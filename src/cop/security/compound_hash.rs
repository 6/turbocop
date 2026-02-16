use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct CompoundHash;

impl Cop for CompoundHash {
    fn name(&self) -> &'static str {
        "Security/CompoundHash"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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

        if call.name().as_slice() != b"hash" {
            return Vec::new();
        }

        // Must have no arguments
        if call.arguments().is_some() {
            return Vec::new();
        }

        // Receiver must be an array literal
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if recv.as_array_node().is_none() {
            return Vec::new();
        }

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `Array#hash` with caution. Consider using a more secure hashing method."
                .to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(CompoundHash, "cops/security/compound_hash");
}
