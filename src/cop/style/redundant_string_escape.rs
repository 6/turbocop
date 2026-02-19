use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::STRING_NODE;

pub struct RedundantStringEscape;

/// Valid escape sequences in double-quoted strings
const MEANINGFUL_ESCAPES: &[u8] = &[
    b'\\', b'\'', b'"', b'a', b'b', b'e', b'f', b'n', b'r', b's', b't', b'v',
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
    b'x', b'u', b'c', b'C', b'M',
    b'#',
    b'\n', b'\r', // literal newline/carriage-return: line continuation in double-quoted strings
];

impl Cop for RedundantStringEscape {
    fn name(&self) -> &'static str {
        "Style/RedundantStringEscape"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Only check double-quoted strings
        let (opening_loc, content_loc) = if let Some(s) = node.as_string_node() {
            match (s.opening_loc(), s.content_loc()) {
                (Some(o), l) => (o, l),
                _ => return Vec::new(),
            }
        } else {
            return Vec::new();
        };

        let open_bytes = opening_loc.as_slice();
        // Must be a double-quoted string
        if open_bytes != b"\"" {
            return Vec::new();
        }

        let content = content_loc.as_slice();
        let content_start = content_loc.start_offset();
        let mut diagnostics = Vec::new();
        let mut i = 0;

        while i < content.len() {
            if content[i] == b'\\' && i + 1 < content.len() {
                let escaped = content[i + 1];
                if !MEANINGFUL_ESCAPES.contains(&escaped) && !escaped.is_ascii_alphabetic() {
                    let abs_offset = content_start + i;
                    let (line, column) = source.offset_to_line_col(abs_offset);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Redundant escape of `{}` in string.", escaped as char),
                    ));
                }
                i += 2;
            } else {
                i += 1;
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantStringEscape, "cops/style/redundant_string_escape");
}
