use crate::cop::node_type::{INTERPOLATED_REGULAR_EXPRESSION_NODE, REGULAR_EXPRESSION_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// ## Investigation (2026-04-01)
///
/// Earlier fixes aligned the cop with RuboCop for escaped `\-` outside character
/// classes, backslash-newline line continuations, and interpolated regexp nodes.
/// The remaining FN cluster came from multiline `/x` interpolated regexps like
/// `%r{ #{url} (https?:\/\/)? }x` and `/#{chars}[\.,]#{chars}/x`: this scanner
/// was still stopping at the first interpolation whenever the regexp contained a
/// newline, so redundant escapes after `#{...}` were never visited.
///
/// The correct narrow behavior is to keep scanning through interpolations for
/// normal regexp literals, but preserve RuboCop's block-call quirk where
/// `rule %r{(#{complex_id})(#{ws}*)([\{\(])}mx do` only reports the literal
/// prefix before the first interpolation. The byte-based scanner and offset
/// mapping stay unchanged otherwise.
///
/// ## Investigation (2026-04-02)
///
/// **FN fix (3 FN):** RuboCop's `requires_escape_to_avoid_interpolation?` checks
/// `node.source[ts]` where `ts` is the offset within regexp content, but
/// `node.source` includes the delimiter prefix. For `/` (prefix 1 char) this
/// accidentally gives `content[ts-1]` (correct). For `%r(` / `%r{` / `%r/`
/// (prefix 3 chars) it gives `content[ts-3]`, effectively disabling the
/// interpolation check. We replicate this offset so that `#\@` and `#\$` in
/// `%r` regexps are flagged as redundant, matching RuboCop.
///
/// **FN fix (5 FN):** A later change added a pre-scan that skipped any
/// interpolated regexp when a string chunk after interpolation began with `+`,
/// `*`, or `?`. Real corpus cases like `/\<#{node}*/`,
/// `%r{(https?:\/\/)(/#{x}*)?}x`, `/(\|[^\|]+\||#{x}#{y}*)/`, and
/// `/(?:#{id}|#{op}+\`[^`]+`)/` are still reported by RuboCop, so that guard
/// was overfit and suppressed legitimate diagnostics. Removing it restores the
/// missing offenses while the narrower block-call quirk remains in place.
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
const INTERPOLATION_SIGILS: &[u8] = b"@$";

#[derive(Clone, Copy)]
struct RegexFlags {
    extended: bool,
    euc_jp: bool,
    windows_31j: bool,
}

impl RegexFlags {
    fn suppresses_all_offenses(self) -> bool {
        self.euc_jp || self.windows_31j
    }
}

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
        let flags = regex_flags(full_bytes);
        let prefix_len = if full_bytes.starts_with(b"%r") { 3 } else { 1 };

        if flags.suppresses_all_offenses() {
            return;
        }

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
                flags,
                prefix_len,
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
            flags,
            prefix_len,
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

fn regex_flags(full_bytes: &[u8]) -> RegexFlags {
    let mut flags = RegexFlags {
        extended: false,
        euc_jp: false,
        windows_31j: false,
    };

    let mut idx = full_bytes.len();
    while idx > 0 && full_bytes[idx - 1].is_ascii_alphabetic() {
        idx -= 1;
        match full_bytes[idx] {
            b'x' => flags.extended = true,
            b'e' => flags.euc_jp = true,
            b's' => flags.windows_31j = true,
            _ => {}
        }
    }

    flags
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

#[allow(clippy::too_many_arguments)]
fn check_regexp_fragment(
    cop: &RedundantRegexpEscape,
    source: &SourceFile,
    content: &[u8],
    offsets: &[Option<usize>],
    delimiter_chars: &[u8],
    flags: RegexFlags,
    prefix_len: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut i = 0;
    let mut char_class_depth = 0usize;

    while i < content.len() {
        let current = content[i];
        if current == INTERPOLATION_BOUNDARY {
            i += 1;
            continue;
        }

        let in_char_class = char_class_depth > 0;

        if flags.extended && !in_char_class && current == b'#' && is_unescaped(content, i) {
            i += 1;
            while i < content.len() && content[i] != b'\n' && content[i] != b'\r' {
                i += 1;
            }
            continue;
        }

        if current == b'[' && is_unescaped(content, i) {
            if in_char_class && let Some(next_i) = skip_named_character_class(content, i) {
                i = next_i;
                continue;
            }
            char_class_depth += 1;
            i += 1;
            continue;
        }

        if current == b']' && in_char_class && is_unescaped(content, i) {
            char_class_depth -= 1;
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
                    let at_start =
                        i > 0 && content[i - 1] == b'[' && (i < 2 || content[i - 2] != b'\\');
                    let at_end = i + 2 < content.len()
                        && content[i + 2] == b']'
                        && is_unescaped(content, i + 2);
                    !(at_start || at_end)
                } else {
                    MEANINGFUL_ESCAPES_IN_CHAR_CLASS.contains(&escaped)
                        || requires_escape_to_avoid_interpolation(content, i, escaped, prefix_len)
                        || escaped.is_ascii_alphabetic()
                        || escaped == b' '
                }
            } else {
                MEANINGFUL_ESCAPES.contains(&escaped)
                    || requires_escape_to_avoid_interpolation(content, i, escaped, prefix_len)
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

fn skip_named_character_class(content: &[u8], start: usize) -> Option<usize> {
    let delimiter = *content.get(start + 1)?;
    if !matches!(delimiter, b':' | b'.' | b'=') {
        return None;
    }

    let mut idx = start + 2;
    while idx + 1 < content.len() {
        if content[idx] == delimiter && content[idx + 1] == b']' {
            return Some(idx + 2);
        }
        idx += 1;
    }

    None
}

fn requires_escape_to_avoid_interpolation(
    content: &[u8],
    index: usize,
    escaped: u8,
    prefix_len: usize,
) -> bool {
    // RuboCop checks `node.source[ts]` where `ts` is the offset in regexp content
    // but `node.source` includes the delimiter prefix. For `/` (prefix_len=1) this
    // gives content[ts-1] (correct). For `%r(` (prefix_len=3) this gives
    // content[ts-3] (a bug that effectively disables the check for %r regexps).
    // We replicate this offset to match RuboCop's behavior.
    let offset = prefix_len.saturating_sub(1);
    index > offset && content[index - 1 - offset] == b'#' && INTERPOLATION_SIGILS.contains(&escaped)
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
