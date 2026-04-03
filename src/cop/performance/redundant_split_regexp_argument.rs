use crate::cop::shared::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-04):
/// - 7 FNs: all involved regex args with flags (/o, /m) that were incorrectly skipped.
/// - Root cause: cop skipped ALL regexes with flags (`closing.len() > 1`), but RuboCop
///   only skips `/i` (case insensitive). Flags /o, /m, /x don't affect simple literal
///   pattern matching in split.
/// - Also fixed: `%r/pattern/` (slash-delimited %r) was incorrectly skipped. RuboCop's
///   DETERMINISTIC_REGEX matches the full source, and `/` is in LITERAL_REGEX, so
///   `%r/pattern/` should be checked. Only `%r{pattern}` and other non-slash delimiters
///   are correctly skipped (braces, brackets, parens not in LITERAL_REGEX).
///
/// ## Extended corpus investigation (2026-03-24)
///
/// Extended corpus reported FP=9, FN=0. All 9 FPs from files containing
/// invalid multibyte regex escapes that crash RuboCop's parser, causing all
/// other cops to be skipped. Not a cop logic issue. Fixed by adding the
/// affected files to `repo_excludes.json`.
pub struct RedundantSplitRegexpArgument;

/// Check if regex content is a simple literal that could be replaced by a string.
/// Returns false for patterns with special regex characters like character classes,
/// quantifiers, alternation, anchors, etc.
///
/// Matches RuboCop's `LITERAL_REGEX` from `lib/rubocop/cop/util.rb`:
///   /[\w\s\-,"'!#%&<>=;:`~/]|\\[^AbBdDgGhHkpPRwWXsSzZ0-9]/
///
/// Unescaped characters matching the first alternation are simple literals.
/// Backslash escapes are simple literals as long as the next character is NOT
/// a special regex class/anchor (`\d`, `\w`, `\A`, `\z`, `\b`, etc.) or a
/// backreference (`\0`-`\9`).
fn is_simple_literal_regex(content: &[u8]) -> bool {
    // Empty regexp // can be replaced with ""
    if content.is_empty() {
        return true;
    }

    // Single space / / is NOT equivalent to " " for split:
    // "  foo  ".split(" ") strips/collapses leading whitespace,
    // "  foo  ".split(/ /) preserves empty strings for each space.
    if content == b" " {
        return false;
    }

    let mut i = 0;
    while i < content.len() {
        let b = content[i];
        if b == b'\\' {
            // Backslash escape sequence
            if i + 1 >= content.len() {
                // Trailing backslash — not a simple literal
                return false;
            }
            let next = content[i + 1];
            // True regex features — NOT simple literals:
            // \A \b \B \d \D \g \G \h \H \k \p \P \R \s \S \w \W \X \z \Z \0-\9
            match next {
                b'A'
                | b'b'
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
                | b's'
                | b'S'
                | b'w'
                | b'W'
                | b'X'
                | b'z'
                | b'Z'
                | b'0'..=b'9' => return false,
                // Everything else after backslash is a simple literal escape
                _ => {
                    i += 2;
                }
            }
        } else {
            // Unescaped characters: check against RuboCop's LITERAL_REGEX character class
            // [\w\s\-,"'!#%&<>=;:`~/]
            // Characters NOT in this class are regex metacharacters.
            match b {
                // \w: word characters (alphanumeric + underscore)
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' => {
                    i += 1;
                }
                // \s: whitespace
                b' ' | b'\t' | b'\n' | b'\r' => {
                    i += 1;
                }
                // Explicitly listed literal characters
                b'-' | b',' | b'"' | b'\'' | b'!' | b'#' | b'%' | b'&' | b'<' | b'>' | b'='
                | b';' | b':' | b'`' | b'~' | b'/' => {
                    i += 1;
                }
                // Anything else (., *, +, ?, |, (, ), [, ], {, }, ^, $, etc.)
                // is a regex metacharacter
                _ => return false,
            }
        }
    }
    true
}

impl Cop for RedundantSplitRegexpArgument {
    fn name(&self) -> &'static str {
        "Performance/RedundantSplitRegexpArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
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

        if call.name().as_slice() != b"split" {
            return;
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return;
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        // RuboCop only flags split with exactly one argument (the regexp).
        // When a limit argument is present (e.g. str.split(/ /, 3)), the
        // regex-to-string replacement may not be equivalent in all edge cases.
        if args.len() != 1 {
            return;
        }

        // Check if first argument is a RegularExpressionNode with simple literal content
        let first_arg = match args.iter().next() {
            Some(a) => a,
            None => return,
        };
        let regex_node = match first_arg.as_regular_expression_node() {
            Some(r) => r,
            None => return,
        };

        // Skip %r syntax with non-slash delimiters — RuboCop's DETERMINISTIC_REGEX
        // matches against the full source (including delimiters), and %r delimiters
        // like {, [, ( are not in its LITERAL_REGEX character class, so they never match.
        // However, %r/pattern/ uses / delimiters which ARE in LITERAL_REGEX, so those
        // should be checked like regular /pattern/ regexps.
        let node_loc = first_arg.location();
        let full_bytes = &source.as_bytes()[node_loc.start_offset()..node_loc.end_offset()];
        if full_bytes.starts_with(b"%r") && !full_bytes.starts_with(b"%r/") {
            return;
        }

        // Skip regexps with /i flag (case insensitive) — changes matching semantics.
        // Other flags like /o (once), /m (multiline), /x (extended) don't affect
        // matching of simple literal patterns in split, so we still flag those.
        // This matches RuboCop's behavior which only checks `ignore_case?`.
        let closing = regex_node.closing_loc().as_slice();
        if closing.contains(&b'i') {
            return;
        }

        let content = regex_node.content_loc().as_slice();
        if !is_simple_literal_regex(content) {
            return;
        }

        let loc = regex_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use string as argument instead of regexp.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        RedundantSplitRegexpArgument,
        "cops/performance/redundant_split_regexp_argument"
    );
}
