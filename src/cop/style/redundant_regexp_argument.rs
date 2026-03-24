use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Style/RedundantRegexpArgument flags regexp arguments that could be strings.
///
/// ## Investigation findings (2026-03-18, updated 2026-03-23)
///
/// ### Root cause: blocklist vs whitelist mismatch
/// RuboCop checks the regexp node's full source (including delimiters like `/`)
/// against `DETERMINISTIC_REGEX = /\A(?:LITERAL_REGEX)+\Z/` where
/// `LITERAL_REGEX = /[\w\s\-,"'!#%&<>=;:`~/]|\\[^AbBdDgGhHkpPRwWXsSzZ0-9]/`.
///
/// Previous implementation used a blocklist (reject known metacharacters, allow
/// everything else), which caused:
/// - **FP (50):** Characters like `@`, `$`, non-ASCII (`ß`, `⌘`) are not regex
///   metacharacters but also not in RuboCop's LITERAL_REGEX whitelist. The blocklist
///   allowed them, producing false positives on patterns like `/@/`, `/ß/`.
/// - **FN (127):** Empty regexp `//` was rejected (empty content check). Also, ALL
///   `%r` variants were skipped, but RuboCop flags `%r/foo/` and `%r!foo!` because
///   `/` and `!` are in the LITERAL_REGEX whitelist (while `{`, `(`, `[`, `|` are not).
///
/// ### Fix: whitelist approach matching RuboCop's LITERAL_REGEX
/// Switched to a whitelist that checks the full source (delimiters + content) against
/// exactly the same character set as RuboCop. This naturally handles all edge cases:
/// - `//` — two `/` chars in whitelist — deterministic (flagged)
/// - `/@/` — `@` not in whitelist — not deterministic (not flagged)
/// - `%r{foo}` — `{` not in whitelist — not deterministic (not flagged)
/// - `%r/foo/` — all chars in whitelist — deterministic (flagged)
/// - `%r!foo!` — all chars in whitelist — deterministic (flagged)
pub struct RedundantRegexpArgument;

/// Methods where a regexp argument can be replaced with a string.
/// Must match vendor RuboCop's RESTRICT_ON_SEND list.
const TARGET_METHODS: &[&[u8]] = &[
    b"byteindex",
    b"byterindex",
    b"gsub",
    b"gsub!",
    b"partition",
    b"rpartition",
    b"scan",
    b"split",
    b"start_with?",
    b"sub",
    b"sub!",
];

impl Cop for RedundantRegexpArgument {
    fn name(&self) -> &'static str {
        "Style/RedundantRegexpArgument"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        if !TARGET_METHODS.contains(&name) {
            return;
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // First argument must be a regexp literal
        let regex = match arg_list[0].as_regular_expression_node() {
            Some(r) => r,
            None => return,
        };

        // Skip if regexp has flags (e.g., /foo/i, /foo/x)
        let closing = regex.closing_loc();
        let close_bytes = closing.as_slice();
        // For /regex/, closing is "/" (len 1). For /regex/i, closing is "/i" (len > 1).
        // For %r/regex/, closing is "/" (len 1). For %r/regex/i, closing is "/i" (len > 1).
        // For %r{regex}, closing is "}" (len 1). For %r{regex}i, closing is "}i" (len > 1).
        if close_bytes.len() > 1 {
            return;
        }

        // Skip single space regexps: / / is idiomatic
        let content = regex.content_loc().as_slice();
        if content == b" " {
            return;
        }

        // Check if the regexp source is deterministic using the full source
        // (including delimiters), matching RuboCop's DETERMINISTIC_REGEX behavior.
        let full_loc = arg_list[0].location();
        let full_source = &source.as_bytes()[full_loc.start_offset()..full_loc.end_offset()];
        if !is_deterministic_regexp_source(full_source) {
            return;
        }

        let loc = arg_list[0].location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use string `\"` instead of regexp `/` as the argument.".to_string(),
        ));
    }
}

/// Check if the full regexp source is deterministic (matches a fixed string).
/// Matches RuboCop's DETERMINISTIC_REGEX = /\A(?:LITERAL_REGEX)+\Z/ where:
///   LITERAL_REGEX = /[\w\s\-,"'!#%&<>=;:`~\/]|\\[^AbBdDgGhHkpPRwWXsSzZ0-9]/
///
/// This checks the full source including delimiters (e.g., `/foo/` or `%r/foo/`),
/// which is how RuboCop applies its regex. The delimiters themselves must also be
/// in the literal character set, which naturally excludes `%r{...}`, `%r(...)`, etc.
fn is_deterministic_regexp_source(source: &[u8]) -> bool {
    if source.is_empty() {
        return false;
    }

    // Regex-special escape chars that indicate a non-deterministic pattern
    const REGEX_ESCAPE_SPECIALS: &[u8] = b"AbBdDgGhHkpPRwWXsSzZ0123456789";

    let mut i = 0;
    while i < source.len() {
        let b = source[i];
        if b == b'\\' {
            // Backslash escape: next char must not be a regex-special escape
            i += 1;
            if i >= source.len() {
                return false; // trailing backslash
            }
            if REGEX_ESCAPE_SPECIALS.contains(&source[i]) {
                return false;
            }
            // Escaped literal char — this is fine (e.g., \., \/, \-)
        } else if is_literal_char(b) {
            // Character is in the LITERAL_REGEX whitelist
        } else {
            return false;
        }
        i += 1;
    }
    true
}

/// Check if a byte is in RuboCop's LITERAL_REGEX unescaped character set:
/// [\w\s\-,"'!#%&<>=;:`~/]
/// This is: word chars (a-z, A-Z, 0-9, _), whitespace (space, \t, \n, \r, \f, \v),
/// and specific punctuation: - , " ' ! # % & < > = ; : ` ~ /
#[inline]
fn is_literal_char(b: u8) -> bool {
    matches!(b,
        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_'  // \w
        | b' ' | b'\t' | b'\n' | b'\r' | 0x0C | 0x0B      // \s (space, tab, newline, CR, FF, VT)
        | b'-' | b',' | b'"' | b'\'' | b'!' | b'#' | b'%' | b'&'
        | b'<' | b'>' | b'=' | b';' | b':' | b'`' | b'~' | b'/'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantRegexpArgument,
        "cops/style/redundant_regexp_argument"
    );
}
