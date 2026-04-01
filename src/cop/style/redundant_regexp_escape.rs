use crate::cop::node_type::{INTERPOLATED_REGULAR_EXPRESSION_NODE, REGULAR_EXPRESSION_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Investigation (2026-04-01)
///
/// The main corpus FN pattern was `\-` outside character classes, e.g.
/// `/\w{8}\-\w{4}/`, which this cop mistakenly treated as meaningful.
/// The main FP pattern was backslash-newline regexp line continuation in multiline
/// regexps, which RuboCop accepts. Interpolated regexps like
/// `/^\[\<assembly: #{attr_name}(.+)/` were also skipped entirely because only
/// `RegularExpressionNode` was visited. This fix preserves the byte-based scanner
/// but removes the outside-char-class `\-` exemption, allows backslash-newline,
/// and scans interpolated regexp string fragments with byte-accurate offsets.
/// It also matches two RuboCop quirks seen in corpus validation:
/// `\-` immediately after `[^` is treated as meaningful, and interpolated
/// regexps that are direct arguments to block calls only report escapes from the
/// literal prefix before the first interpolation.
pub struct RedundantRegexpEscape;

/// Characters that need escaping OUTSIDE a character class in regexp
const MEANINGFUL_ESCAPES: &[u8] = b".|()[]{}*+?\\^$#ntrfaevbBsSdDwWhHAzZGpPRXkg0123456789xucCM";

/// Characters that need escaping INSIDE a character class `[...]`.
/// Inside a class, metacharacters like `.`, `(`, `)`, `*`, `+`, `?`, `|`, `{`, `}`
/// are literal and don't need escaping. Only `]`, `\`, `^`, `-` are special.
/// Note: `#` is always allowed to be escaped (to prevent interpolation ambiguity).
/// Note: `\-` is only meaningful if NOT at the start/end of the class; this is
/// handled separately in the check logic below.
const MEANINGFUL_ESCAPES_IN_CHAR_CLASS: &[u8] = b"\\]^[#ntrfaevbBsSdDwWhHAzZGpPRXkg0123456789xucCM";
const INTERPOLATION_BOUNDARY: u8 = 0;

impl Cop for RedundantRegexpEscape {
    fn name(&self) -> &'static str {
        "Style/RedundantRegexpEscape"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            REGULAR_EXPRESSION_NODE,
            INTERPOLATED_REGULAR_EXPRESSION_NODE,
        ]
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
        let node_loc = node.location();
        let full_bytes = &source.as_bytes()[node_loc.start_offset()..node_loc.end_offset()];
        let delimiter_chars = delimiter_chars(full_bytes);

        if let Some(re) = node.as_regular_expression_node() {
            let content = re.content_loc().as_slice();
            let offsets = (0..content.len())
                .map(|idx| Some(re.content_loc().start_offset() + idx))
                .collect::<Vec<_>>();
            check_regexp_fragment(
                self,
                source,
                content,
                &offsets,
                &delimiter_chars,
                diagnostics,
            );
            return;
        }

        let Some(re) = node.as_interpolated_regular_expression_node() else {
            return;
        };

        let mut content = Vec::new();
        let mut offsets = Vec::new();

        let scan_full_interpolated =
            !followed_by_block_opener(source.as_bytes(), node_loc.end_offset());

        for part in re.parts().iter() {
            if let Some(string) = part.as_string_node() {
                append_bytes_with_offsets(
                    &mut content,
                    &mut offsets,
                    string.content_loc().as_slice(),
                    string.content_loc().start_offset(),
                );
            } else {
                if !scan_full_interpolated {
                    break;
                }
                content.push(INTERPOLATION_BOUNDARY);
                offsets.push(None);
            }
        }

        check_regexp_fragment(
            self,
            source,
            &content,
            &offsets,
            &delimiter_chars,
            diagnostics,
        );
    }
}

fn delimiter_chars(full_bytes: &[u8]) -> Vec<u8> {
    if full_bytes.starts_with(b"%r") && full_bytes.len() >= 3 {
        match full_bytes[2] {
            b'(' => vec![b'(', b')'],
            b'{' => vec![b'{', b'}'],
            b'[' => vec![b'[', b']'],
            b'<' => vec![b'<', b'>'],
            delim => vec![delim],
        }
    } else {
        vec![b'/']
    }
}

fn followed_by_block_opener(source: &[u8], mut offset: usize) -> bool {
    while offset < source.len() && source[offset].is_ascii_whitespace() {
        offset += 1;
    }

    if offset >= source.len() {
        return false;
    }

    if source[offset] == b'{' {
        return true;
    }

    if source[offset..].starts_with(b"do") {
        let next = source.get(offset + 2).copied();
        return next.is_none_or(|byte| !byte.is_ascii_alphanumeric() && byte != b'_');
    }

    false
}

fn append_bytes_with_offsets(
    content: &mut Vec<u8>,
    offsets: &mut Vec<Option<usize>>,
    bytes: &[u8],
    start_offset: usize,
) {
    for (idx, byte) in bytes.iter().copied().enumerate() {
        content.push(byte);
        offsets.push(Some(start_offset + idx));
    }
}

fn check_regexp_fragment(
    cop: &RedundantRegexpEscape,
    source: &SourceFile,
    content: &[u8],
    offsets: &[Option<usize>],
    delimiter_chars: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut i = 0;
    let mut in_char_class = false;
    let mut char_class_start = 0usize;

    while i < content.len() {
        let current = content[i];
        if current == INTERPOLATION_BOUNDARY {
            i += 1;
            continue;
        }

        if current == b'[' && is_unescaped(content, i) {
            in_char_class = true;
            char_class_start = i;
            i += 1;
            if i < content.len() && content[i] == b'^' {
                i += 1;
            }
            continue;
        }

        if current == b']' && in_char_class && is_unescaped(content, i) {
            in_char_class = false;
            i += 1;
            continue;
        }

        if current == b'\\' && i + 1 < content.len() {
            let escaped = content[i + 1];
            if escaped == INTERPOLATION_BOUNDARY {
                i += 1;
                continue;
            }

            if escaped == b'\n' {
                i += 2;
                continue;
            }

            if escaped == b'\r' {
                i += 2;
                if i < content.len() && content[i] == b'\n' {
                    i += 1;
                }
                continue;
            }

            let is_meaningful = if in_char_class {
                if escaped == b'-' {
                    let at_start = i == char_class_start + 1;
                    let at_end = i + 2 < content.len()
                        && content[i + 2] == b']'
                        && is_unescaped(content, i + 2);
                    !(at_start || at_end)
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

            if !is_meaningful && !delimiter_chars.contains(&escaped) {
                let Some(abs_offset) = offsets.get(i).copied().flatten() else {
                    i += 2;
                    continue;
                };
                let (line, column) = source.offset_to_line_col(abs_offset);
                diagnostics.push(cop.diagnostic(
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

fn is_unescaped(content: &[u8], idx: usize) -> bool {
    let mut backslashes = 0usize;
    let mut cursor = idx;

    while cursor > 0 {
        let prev = content[cursor - 1];
        if prev == INTERPOLATION_BOUNDARY || prev != b'\\' {
            break;
        }
        backslashes += 1;
        cursor -= 1;
    }

    backslashes % 2 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantRegexpEscape, "cops/style/redundant_regexp_escape");
}
