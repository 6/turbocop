use crate::cop::shared::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Investigation (2026-03-04)
///
/// 10 FNs in jruby and natalie repos, all involving `\c` and `\C-` control
/// character escapes in regex patterns (e.g., `/\c(\cH\ch/.match(str)`).
///
/// **Root cause:** RuboCop's parser gem pre-interprets regex content, so `\c(`
/// becomes byte `\x08` before `LITERAL_REGEX` checks it. Prism gives raw source,
/// so `is_literal_regex()` saw `(` after `\c` and rejected it as non-literal.
///
/// **Fix:** Added handling for `\cX` (3-byte) and `\C-X`/`\M-X` (4-byte) control
/// and meta character escapes in `is_literal_regex()`, treating them as literal
/// character sequences.
///
/// ## Extended corpus investigation (2026-03-24)
///
/// Extended corpus reported FP=4, FN=0. All 4 FPs from files containing
/// invalid multibyte regex escapes that crash RuboCop's parser, causing all
/// other cops to be skipped. Not a cop logic issue. Fixed by adding the
/// affected files to `repo_excludes.json`.
pub struct StringInclude;

/// Check if a single byte is in RuboCop's literal character allowlist.
/// Matches: `[\w\s\-,"'!#%&<>=;:`~/]` from RuboCop's `Util::LITERAL_REGEX`.
fn is_literal_char(b: u8) -> bool {
    match b {
        // \w: [a-zA-Z0-9_]
        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => true,
        // \s: [ \t\n\r\f] (form feed = 0x0C)
        b' ' | b'\t' | b'\n' | b'\r' | 0x0C => true,
        // Explicit punctuation from LITERAL_REGEX
        b'-' | b',' | b'"' | b'\'' | b'!' | b'#' | b'%' | b'&' | b'<' | b'>' | b'=' | b';'
        | b':' | b'`' | b'~' | b'/' => true,
        _ => false,
    }
}

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

/// Check if a regex pattern (raw content between slashes) contains only
/// characters that RuboCop considers literal — matching the allowlist in
/// `Util::LITERAL_REGEX`.
///
/// RuboCop's parser gem pre-interprets regex escape sequences (e.g., `\c(`
/// becomes `\x08`), so its LITERAL_REGEX only sees plain bytes. Prism gives
/// us raw source, so we must also accept Ruby regex control-char escapes:
/// - `\cX` (3 bytes: `\`, `c`, any char) — control character
/// - `\C-X` (4 bytes: `\`, `C`, `-`, any char) — control character
/// - `\M-X` (4 bytes: `\`, `M`, `-`, any char) — meta character
/// - `\M-\C-X` / `\M-\cX` (nested meta+control combos)
fn is_literal_regex(content: &[u8]) -> bool {
    if content.is_empty() {
        return false;
    }
    let mut i = 0;
    while i < content.len() {
        if content[i] == b'\\' {
            if i + 1 >= content.len() {
                return false;
            }
            let next = content[i + 1];
            if next == b'c' {
                // \cX — control char escape, consumes 3 bytes total
                if i + 2 >= content.len() {
                    return false;
                }
                i += 3;
            } else if (next == b'C' || next == b'M')
                && i + 2 < content.len()
                && content[i + 2] == b'-'
            {
                // \C-X or \M-X — control/meta char escape, consumes 4 bytes total
                if i + 3 >= content.len() {
                    return false;
                }
                i += 4;
            } else if is_regex_escape_metachar(next) {
                return false;
            } else {
                // Simple backslash escape of a non-metachar (e.g., `\.`, `\t`, `\n`)
                i += 2;
            }
        } else if is_literal_char(content[i]) {
            i += 1;
        } else {
            return false;
        }
    }
    true
}

/// Check if a node is a regex literal with no flags and a literal-only pattern.
fn is_simple_regex_node(node: &ruby_prism::Node<'_>) -> bool {
    let regex_node = match node.as_regular_expression_node() {
        Some(r) => r,
        None => return false,
    };
    // Skip if regex has flags (e.g., /pattern/i)
    let closing = regex_node.closing_loc().as_slice();
    if closing.len() > 1 {
        return false;
    }
    let content = regex_node.content_loc().as_slice();
    is_literal_regex(content)
}

impl Cop for StringInclude {
    fn name(&self) -> &'static str {
        "Performance/StringInclude"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, REGULAR_EXPRESSION_NODE]
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

        let name = call.name().as_slice();

        let is_match = match name {
            // str.match?(/regex/) or /regex/.match?(str) or str.match(/regex/) or /regex/.match(str)
            b"match?" | b"match" => {
                if call.receiver().is_none() {
                    return;
                }
                let arguments = match call.arguments() {
                    Some(a) => a,
                    None => return,
                };
                let args: Vec<_> = arguments.arguments().iter().collect();
                // RuboCop only matches single-argument calls; match(re, pos) is not flagged
                if args.len() != 1 {
                    return;
                }
                let recv = call.receiver().unwrap();

                // Either the argument or the receiver must be a simple regex
                is_simple_regex_node(&args[0]) || is_simple_regex_node(&recv)
            }

            // /regex/ === str
            b"===" => {
                let recv = match call.receiver() {
                    Some(r) => r,
                    None => return,
                };
                is_simple_regex_node(&recv)
            }

            // str =~ /regex/ or /regex/ =~ str (both directions)
            b"=~" => {
                let recv = match call.receiver() {
                    Some(r) => r,
                    None => return,
                };
                let arguments = match call.arguments() {
                    Some(a) => a,
                    None => return,
                };
                let first_arg = match arguments.arguments().iter().next() {
                    Some(a) => a,
                    None => return,
                };
                is_simple_regex_node(&recv) || is_simple_regex_node(&first_arg)
            }

            // str !~ /regex/ (regex as argument only; /regex/ !~ str is NOT flagged by RuboCop)
            b"!~" => {
                let arguments = match call.arguments() {
                    Some(a) => a,
                    None => return,
                };
                let first_arg = match arguments.arguments().iter().next() {
                    Some(a) => a,
                    None => return,
                };
                is_simple_regex_node(&first_arg)
            }

            _ => return,
        };

        if !is_match {
            return;
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `String#include?` instead of a regex match with literal-only pattern.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(StringInclude, "cops/performance/string_include");
}
