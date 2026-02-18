use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantRegexpEscape;

/// Characters that need escaping in regexp
const MEANINGFUL_ESCAPES: &[u8] = &[
    b'.', b'|', b'(', b')', b'[', b']', b'{', b'}', b'*', b'+', b'?', b'\\',
    b'^', b'$', b'-', b'#',
    // Escape sequences
    b'n', b't', b'r', b'f', b'a', b'e', b'v', b'b', b'B',
    b's', b'S', b'd', b'D', b'w', b'W', b'h', b'H',
    b'A', b'z', b'Z', b'G', b'p', b'P', b'R', b'X',
    b'k', b'g',
    // Numeric escapes
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',
    b'x', b'u', b'c', b'C', b'M',
];

impl Cop for RedundantRegexpEscape {
    fn name(&self) -> &'static str {
        "Style/RedundantRegexpEscape"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let re = match node.as_regular_expression_node() {
            Some(re) => re,
            None => return Vec::new(),
        };
        let content: Vec<u8> = re.content_loc().as_slice().to_vec();
        let node_loc = node.location();

        let full_bytes = &source.as_bytes()[node_loc.start_offset()..node_loc.end_offset()];
        let open_len = if full_bytes.starts_with(b"%r") { 3 } else { 1 };

        let mut diagnostics = Vec::new();
        let mut i = 0;
        let mut in_char_class = false;

        while i < content.len() {
            if content[i] == b'[' && (i == 0 || content[i - 1] != b'\\') {
                in_char_class = true;
                i += 1;
                continue;
            }
            if content[i] == b']' && in_char_class {
                in_char_class = false;
                i += 1;
                continue;
            }

            if content[i] == b'\\' && i + 1 < content.len() {
                let escaped = content[i + 1];
                if !MEANINGFUL_ESCAPES.contains(&escaped)
                    && !escaped.is_ascii_alphabetic()
                    && escaped != b' '
                {
                    // Also allow escaping / in slash-delimited regexp
                    if escaped == b'/' {
                        i += 2;
                        continue;
                    }

                    let abs_offset = node_loc.start_offset() + open_len + i;
                    let (line, column) = source.offset_to_line_col(abs_offset);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Redundant escape of `{}` in regexp.", escaped as char),
                    ));
                }
                i += 2;
                continue;
            }
            i += 1;
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantRegexpEscape, "cops/style/redundant_regexp_escape");
}
