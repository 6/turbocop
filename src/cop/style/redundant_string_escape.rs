use crate::cop::node_type::STRING_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantStringEscape;

/// Valid escape sequences in double-quoted strings
const MEANINGFUL_ESCAPES: &[u8] = b"\\'\"abefnrstv01234567xucCM#\n\r";

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
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Only check double-quoted strings
        let (opening_loc, content_loc) = if let Some(s) = node.as_string_node() {
            match (s.opening_loc(), s.content_loc()) {
                (Some(o), l) => (o, l),
                _ => return,
            }
        } else {
            return;
        };

        let open_bytes = opening_loc.as_slice();
        // Must be a double-quoted string
        if open_bytes != b"\"" {
            return;
        }

        let content = content_loc.as_slice();
        let content_start = content_loc.start_offset();
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantStringEscape, "cops/style/redundant_string_escape");
}
