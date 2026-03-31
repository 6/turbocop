use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Detects redundant line continuations (`\` at end of line).
///
/// This cop now mirrors RuboCop's Ruby 4.0 baseline behavior more closely by
/// removing each trailing `\` and reparsing the file instead of relying on a
/// fixed operator allowlist. That fixes the large FN cluster where boolean and
/// modifier continuations such as `value \` + `&& other` or `unless foo \` +
/// `|| bar` are redundant under the corpus baseline. We still preserve the
/// narrow cases where RuboCop requires `\`, chiefly string concatenation,
/// arithmetic-leading next lines, unparenthesized method arguments, and
/// leading-dot chains separated by a blank line.
pub struct RedundantLineContinuation;

impl Cop for RedundantLineContinuation {
    fn name(&self) -> &'static str {
        "Style/RedundantLineContinuation"
    }

    fn check_source(
        &self,
        source: &SourceFile,
        _parse_result: &ruby_prism::ParseResult<'_>,
        code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let lines: Vec<&[u8]> = source.lines().collect();
        let source_bytes = source.as_bytes();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = trim_end(line);
            if !trimmed.ends_with(b"\\") {
                continue;
            }

            // Check the character before backslash is not another backslash (string escape)
            if trimmed.len() >= 2 && trimmed[trimmed.len() - 2] == b'\\' {
                continue;
            }

            // Compute the absolute offset of the backslash to check if it's in code.
            let line_start = source.line_start_offset(i + 1);
            let backslash_offset = line_start + trimmed.len() - 1;

            // Use code_map to verify the backslash is in a code region
            // (not inside a string, heredoc, or comment)
            if !code_map.is_code(backslash_offset) {
                continue;
            }

            let before_backslash = trim_end(&trimmed[..trimmed.len() - 1]);

            if continuation_is_required(before_backslash, i, &lines) {
                continue;
            }

            let next_trimmed = lines.get(i + 1).map(|next_line| trim_start(next_line));

            if next_trimmed.is_some_and(starts_with_boolean_operator)
                || is_redundant_continuation(source_bytes, backslash_offset)
            {
                let col = trimmed.len() - 1;
                diagnostics.push(self.diagnostic(
                    source,
                    i + 1,
                    col,
                    "Redundant line continuation.".to_string(),
                ));
            }
        }
    }
}

fn trim_end(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
        end -= 1;
    }
    &bytes[..end]
}

fn trim_start(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < bytes.len() && (bytes[start] == b' ' || bytes[start] == b'\t') {
        start += 1;
    }
    &bytes[start..]
}

fn continuation_is_required(before_backslash: &[u8], line_idx: usize, lines: &[&[u8]]) -> bool {
    let trimmed = trim_end(before_backslash);
    if trimmed.is_empty() {
        return false;
    }

    if string_concatenation(trimmed) {
        return true;
    }

    if leading_dot_method_chain_with_blank_line(trimmed, line_idx, lines) {
        return true;
    }

    let Some(next_line) = lines.get(line_idx + 1) else {
        return false;
    };
    let next_trimmed = trim_start(next_line);

    assignment_to_multiline_rhs(trimmed, next_trimmed)
        || starts_with_arithmetic_operator(next_trimmed)
        || method_with_argument(trimmed, next_trimmed)
}

fn is_redundant_continuation(source: &[u8], backslash_offset: usize) -> bool {
    let mut modified = source.to_vec();
    modified.remove(backslash_offset);
    ruby_prism::parse(&modified).errors().next().is_none()
}

fn string_concatenation(trimmed: &[u8]) -> bool {
    matches!(trimmed.last(), Some(b'"' | b'\''))
}

fn assignment_to_multiline_rhs(before_backslash: &[u8], next_trimmed: &[u8]) -> bool {
    ends_with_assignment_operator(before_backslash)
        && (continues_union_rhs(next_trimmed) || continues_string_concat_rhs(next_trimmed))
}

fn ends_with_assignment_operator(trimmed: &[u8]) -> bool {
    if !trimmed.ends_with(b"=") {
        return false;
    }

    !matches!(
        trimmed.get(trimmed.len().saturating_sub(2)).copied(),
        Some(b'=' | b'!' | b'<' | b'>')
    )
}

fn continues_union_rhs(line: &[u8]) -> bool {
    trim_end(line).ends_with(b"|")
}

fn continues_string_concat_rhs(line: &[u8]) -> bool {
    let trimmed = trim_end(line);
    starts_with_string_literal(trim_start(trimmed)) && trimmed.ends_with(b"+")
}

fn starts_with_string_literal(trimmed: &[u8]) -> bool {
    matches!(trimmed.first(), Some(b'"' | b'\'' | b'`'))
}

fn leading_dot_method_chain_with_blank_line(
    before_backslash: &[u8],
    line_idx: usize,
    lines: &[&[u8]],
) -> bool {
    let trimmed = trim_start(before_backslash);
    if !(trimmed.starts_with(b".") || trimmed.starts_with(b"&.")) {
        return false;
    }

    lines
        .get(line_idx + 1)
        .is_some_and(|next_line| trim_start(next_line).is_empty())
}

fn starts_with_arithmetic_operator(next_trimmed: &[u8]) -> bool {
    next_trimmed.starts_with(b"**")
        || matches!(next_trimmed.first(), Some(b'*' | b'/' | b'%' | b'+' | b'-'))
}

fn starts_with_boolean_operator(next_trimmed: &[u8]) -> bool {
    next_trimmed.starts_with(b"&&") || next_trimmed.starts_with(b"||")
}

fn method_with_argument(before_backslash: &[u8], next_trimmed: &[u8]) -> bool {
    last_token_can_take_argument(before_backslash) && next_line_starts_with_argument(next_trimmed)
}

fn last_token_can_take_argument(before_backslash: &[u8]) -> bool {
    let Some(token) = trailing_identifier(before_backslash) else {
        return false;
    };

    matches!(token, b"break" | b"next" | b"return" | b"super" | b"yield")
        || token
            .first()
            .is_some_and(|b| (b.is_ascii_lowercase() || *b == b'_') && token != b"do")
}

fn trailing_identifier(bytes: &[u8]) -> Option<&[u8]> {
    let end = bytes.len();
    if end > 0 {
        let b = bytes[end - 1];
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'?' | b'!') {
            // Continue below to collect the full trailing identifier token.
        } else {
            return None;
        }
    }

    let mut start = end;
    while start > 0 {
        let b = bytes[start - 1];
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'?' | b'!') {
            start -= 1;
        } else {
            break;
        }
    }

    (start < end).then_some(&bytes[start..end])
}

fn next_line_starts_with_argument(next_trimmed: &[u8]) -> bool {
    if next_trimmed.is_empty() {
        return false;
    }

    if starts_with_boolean_operator(next_trimmed) {
        return false;
    }

    if next_trimmed.starts_with(b"...")
        || next_trimmed.starts_with(b"..")
        || next_trimmed.starts_with(b"->")
        || next_trimmed.starts_with(b"**")
        || next_trimmed.starts_with(b"::")
    {
        return true;
    }

    matches!(
        next_trimmed[0],
        b'"'
            | b'\''
            | b'`'
            | b':'
            | b'?'
            | b'!'
            | b'~'
            | b'['
            | b'{'
            | b'('
            | b'|'
            | b'/'
            | b'*'
            | b'&'
            | b'+'
            | b'-'
            | b'%'
            | b'@'
            | b'$'
            | b'0'..=b'9'
            | b'A'..=b'Z'
            | b'_'
            | b'a'..=b'z'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantLineContinuation,
        "cops/style/redundant_line_continuation"
    );
}
