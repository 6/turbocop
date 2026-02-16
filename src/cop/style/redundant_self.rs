use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantSelf;

/// Methods where self. is always required (keywords, operators, etc.)
const ALLOWED_METHODS: &[&[u8]] = &[
    b"class", b"module", b"def", b"end", b"begin", b"rescue", b"ensure",
    b"if", b"unless", b"while", b"until", b"for", b"do", b"return",
    b"yield", b"super", b"raise", b"defined?",
];

impl Cop for RedundantSelf {
    fn name(&self) -> &'static str {
        "Style/RedundantSelf"
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

        // Must have `self` as receiver
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        if receiver.as_self_node().is_none() {
            return Vec::new();
        }

        // Must use `.` as call operator (not `::` or `&.`)
        let call_op = match call_node.call_operator_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        if call_op.as_slice() != b"." {
            return Vec::new();
        }

        let method_name = call_node.name();
        let name_bytes = method_name.as_slice();

        // Skip if method ends with `=` (setter method requires self)
        if name_bytes.ends_with(b"=") {
            return Vec::new();
        }

        // Skip if method is an operator
        if name_bytes == b"[]" || name_bytes == b"[]=" {
            return Vec::new();
        }

        // Skip keyword-like methods
        if ALLOWED_METHODS.iter().any(|&m| m == name_bytes) {
            return Vec::new();
        }

        let self_loc = receiver.location();
        let (line, column) = source.offset_to_line_col(self_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Redundant `self` detected.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantSelf, "cops/style/redundant_self");
}
