use crate::cop::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Squeeze;

/// Characters that, when preceded by a backslash, form a regex metachar class
/// (e.g., `\d`, `\s`, `\A`). Escaped chars NOT in this set are just literals.
/// Matches: `\\[^AbBdDgGhHkpPRwWXsSzZ0-9]` from RuboCop's `Util::LITERAL_REGEX`.
fn is_regex_escape_metachar(b: u8) -> bool {
    matches!(
        b,
        b'A' | b'b'
            | b'B'
            | b'd'
            | b'D'
            | b'g'
            | b'G'
            | b'h'
            | b'H'
            | b'k'
            | b'p'
            | b'P'
            | b'R'
            | b'w'
            | b'W'
            | b'X'
            | b's'
            | b'S'
            | b'z'
            | b'Z'
            | b'0'..=b'9'
    )
}

/// Interpret a single regex escape sequence (e.g., `\n` → newline byte).
fn interpret_regex_escape(b: u8) -> u8 {
    match b {
        b'n' => b'\n',
        b't' => b'\t',
        b'r' => b'\r',
        b'f' => 0x0C,
        b'a' => 0x07,
        b'e' => 0x1B,
        // For non-special escapes, the char is itself (e.g., `\-` → `-`)
        other => other,
    }
}

/// Check if a single byte is a regex metacharacter (not counting backslash escapes).
fn is_regex_metachar(b: u8) -> bool {
    matches!(
        b,
        b'.' | b'*'
            | b'+'
            | b'?'
            | b'|'
            | b'('
            | b')'
            | b'['
            | b']'
            | b'{'
            | b'}'
            | b'^'
            | b'$'
            | b'\\'
    )
}

/// Extract the single repeated character from a regex pattern like `X+` or `\X+`.
/// Returns the byte value the pattern represents (after interpreting escapes),
/// or None if the pattern isn't a simple single-char-plus-quantifier.
fn extract_repeat_char(regex_content: &[u8]) -> Option<u8> {
    // Must end with '+'
    if regex_content.is_empty() || regex_content[regex_content.len() - 1] != b'+' {
        return None;
    }

    let pattern = &regex_content[..regex_content.len() - 1]; // strip trailing '+'

    if pattern.len() == 1 {
        // Simple single char: must not be a metacharacter
        let ch = pattern[0];
        if is_regex_metachar(ch) {
            return None;
        }
        Some(ch)
    } else if pattern.len() == 2 && pattern[0] == b'\\' {
        // Escape sequence: `\X` where X must NOT be a metachar class
        let escaped = pattern[1];
        if is_regex_escape_metachar(escaped) {
            return None;
        }
        Some(interpret_regex_escape(escaped))
    } else {
        None
    }
}

impl Cop for Squeeze {
    fn name(&self) -> &'static str {
        "Performance/Squeeze"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE]
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();
        let is_bang = match method_name {
            b"gsub" => false,
            b"gsub!" => true,
            _ => return,
        };

        if call.receiver().is_none() {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 2 {
            return;
        }

        let mut iter = args.iter();
        let first_arg = iter.next().unwrap();
        let second_arg = iter.next().unwrap();

        // First arg must be a regex
        let regex_node = match first_arg.as_regular_expression_node() {
            Some(r) => r,
            None => return,
        };

        // Regex must have no flags (e.g., /a+/i is not equivalent to squeeze)
        let closing = regex_node.closing_loc().as_slice();
        if closing.len() > 1 {
            return;
        }

        let regex_content = regex_node.content_loc().as_slice();

        // Extract the single repeated character from the pattern
        let repeat_char = match extract_repeat_char(regex_content) {
            Some(ch) => ch,
            None => return,
        };

        // Second arg must be a single-char string matching the same character
        let string_node = match second_arg.as_string_node() {
            Some(s) => s,
            None => return,
        };

        let replacement = string_node.unescaped();
        if replacement.len() != 1 || replacement[0] != repeat_char {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        let message = if is_bang {
            "Use `squeeze!` instead of `gsub!`.".to_string()
        } else {
            "Use `squeeze` instead of `gsub`.".to_string()
        };
        diagnostics.push(self.diagnostic(source, line, column, message));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Squeeze, "cops/performance/squeeze");
}
