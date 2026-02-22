use crate::cop::node_type::REGULAR_EXPRESSION_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantRegexpEscape;

/// Characters that need escaping OUTSIDE a character class in regexp
const MEANINGFUL_ESCAPES: &[u8] = &[
    b'.', b'|', b'(', b')', b'[', b']', b'{', b'}', b'*', b'+', b'?', b'\\', b'^', b'$', b'-',
    b'#', // Escape sequences
    b'n', b't', b'r', b'f', b'a', b'e', b'v', b'b', b'B', b's', b'S', b'd', b'D', b'w', b'W', b'h',
    b'H', b'A', b'z', b'Z', b'G', b'p', b'P', b'R', b'X', b'k', b'g', // Numeric escapes
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'x', b'u', b'c', b'C', b'M',
];

/// Characters that need escaping INSIDE a character class `[...]`.
/// Inside a class, metacharacters like `.`, `(`, `)`, `*`, `+`, `?`, `|`, `{`, `}`
/// are literal and don't need escaping. Only `]`, `\`, `^`, `-` are special.
/// Note: `#` is always allowed to be escaped (to prevent interpolation ambiguity).
/// Note: `\-` is only meaningful if NOT at the start/end of the class; this is
/// handled separately in the check logic below.
const MEANINGFUL_ESCAPES_IN_CHAR_CLASS: &[u8] = &[
    b'\\', b']', b'^', b'[', b'#',
    // Escape sequences are still meaningful inside character classes
    b'n', b't', b'r', b'f', b'a', b'e', b'v', b'b', b'B', b's', b'S', b'd', b'D', b'w', b'W', b'h',
    b'H', b'A', b'z', b'Z', b'G', b'p', b'P', b'R', b'X', b'k', b'g', // Numeric escapes
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'x', b'u', b'c', b'C', b'M',
];

impl Cop for RedundantRegexpEscape {
    fn name(&self) -> &'static str {
        "Style/RedundantRegexpEscape"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[REGULAR_EXPRESSION_NODE]
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
        let re = match node.as_regular_expression_node() {
            Some(re) => re,
            None => return,
        };
        let content: Vec<u8> = re.content_loc().as_slice().to_vec();
        let node_loc = node.location();

        let full_bytes = &source.as_bytes()[node_loc.start_offset()..node_loc.end_offset()];
        let open_len = if full_bytes.starts_with(b"%r") { 3 } else { 1 };

        // For %r(...) style regexps, escaped delimiter characters are NOT redundant
        // because they prevent the parser from treating them as the delimiter boundary.
        // Determine the opening and closing delimiter characters.
        let delimiter_chars: Vec<u8> = if full_bytes.starts_with(b"%r") && full_bytes.len() >= 3 {
            match full_bytes[2] {
                b'(' => vec![b'(', b')'],
                b'{' => vec![b'{', b'}'],
                b'[' => vec![b'[', b']'],
                b'<' => vec![b'<', b'>'],
                delim => vec![delim],
            }
        } else {
            Vec::new()
        };

        let mut i = 0;
        let mut in_char_class = false;
        let mut char_class_start: usize = 0;

        while i < content.len() {
            if content[i] == b'[' && (i == 0 || content[i - 1] != b'\\') {
                in_char_class = true;
                char_class_start = i;
                i += 1;
                // Skip ^ for negated char classes
                if i < content.len() && content[i] == b'^' {
                    i += 1;
                }
                continue;
            }
            if content[i] == b']' && in_char_class {
                in_char_class = false;
                i += 1;
                continue;
            }

            if content[i] == b'\\' && i + 1 < content.len() {
                let escaped = content[i + 1];

                let is_meaningful = if in_char_class {
                    if escaped == b'-' {
                        // \- is meaningful inside char class UNLESS at start or end.
                        // Check if at start: right after [ or [^
                        let at_start = i == char_class_start + 1
                            || (i == char_class_start + 2 && content[char_class_start + 1] == b'^');
                        // Check if at end: \-] pattern
                        let at_end = i + 2 < content.len() && content[i + 2] == b']';
                        !(at_start || at_end) // meaningful only when NOT at start/end
                    } else {
                        MEANINGFUL_ESCAPES_IN_CHAR_CLASS.contains(&escaped)
                            || escaped.is_ascii_alphabetic()
                            || escaped == b' '
                    }
                } else {
                    MEANINGFUL_ESCAPES.contains(&escaped)
                        || escaped.is_ascii_alphabetic()
                        || escaped == b' '
                };

                if !is_meaningful {
                    // Also allow escaping / in slash-delimited regexp
                    if escaped == b'/' {
                        i += 2;
                        continue;
                    }

                    // Allow escaping delimiter characters in %r(...) style regexps
                    if delimiter_chars.contains(&escaped) {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantRegexpEscape, "cops/style/redundant_regexp_escape");
}
