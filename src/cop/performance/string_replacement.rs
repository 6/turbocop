use crate::cop::shared::node_type::{CALL_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Performance/StringReplacement
///
/// Identifies places where `gsub`/`gsub!` can be replaced by `tr`/`delete`.
///
/// Investigation notes:
/// - Original implementation only handled `gsub` (not `gsub!`) and only single-byte chars.
/// - Root cause of 1054 FNs: byte length check (`len() != 1`) rejected multi-byte UTF-8
///   single characters (e.g., "Á" is 2 bytes but 1 char). Also missed empty replacement
///   pattern (→ `delete`) and `gsub!` (→ `tr!`/`delete!`).
/// - RuboCop only flags `gsub`/`gsub!`, NOT `sub`/`sub!`.
/// - Message format: "Use `tr` instead of `gsub`." / "Use `delete` instead of `gsub`."
///   with bang variants for `gsub!`.
/// - FN fix: removed receiver().is_none() guard — RuboCop flags bare gsub (implicit self).
/// - FN fix: added regex literal handling. RuboCop's `DETERMINISTIC_REGEX` accepts regex
///   args that are simple single-char literals (no metacharacters, no flags, no char classes).
///   Escapes like `\t`, `\n`, `\u00A0` are fine (they represent literal chars).
/// - FP fix (14 FPs): `is_deterministic_single_char_regex` accepted any non-metachar byte,
///   but RuboCop's `LITERAL_REGEX` only allows `[\w\s\-,"'!#%&<>=;:\`~/]` plus escaped chars.
///   Characters like `@` and non-ASCII (e.g., unicode curly quote U+2019) are NOT in the
///   whitelist, so RuboCop does not flag them. Fixed by restricting the regular char branch
///   to only match the LITERAL_REGEX whitelist and rejecting non-ASCII bytes.
///
/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// FP=2: Both from `gsub(/\0/, ...)` — null byte regex escape. RuboCop's LITERAL_REGEX
/// excludes digits 0-9 from valid escape chars: `\\[^AbBdDgGhHkpPRwWXsSzZ0-9]`. Our
/// `is_deterministic_single_char_regex` was missing `0-9`, `g`, `k`, `X` from its exclusion
/// list. Fixed by adding those to the match arm that rejects non-literal escape sequences.
///
/// ## Corpus investigation (2026-03-20)
///
/// Extended corpus oracle reported FP=4 (3 cop logic, 1 vendored-path infrastructure).
///
/// FP=3: Braced unicode escapes `\u{2028}`, `\u{000c}`, `\u{a0}` in regex patterns.
/// RuboCop's LITERAL_REGEX only accepts `\\u[\da-fA-F]{4}` (4-digit form `\uXXXX`),
/// not the braced form `\u{XXXX}`. Fixed by returning false for braced unicode escapes
/// in `is_deterministic_single_char_regex`.
///
/// FP=1: `gsub( / /, "_" )` in `databasically__lowdown` from vendored path
/// (`vendor/rails/actionmailer/lib/action_mailer/quoting.rb`). The file
/// contains invalid multibyte regex escapes (`/[\177-\377]/`) that crash
/// RuboCop's parser, causing all other cops to be skipped. Not a cop logic
/// issue. Fixed by adding the file to `repo_excludes.json`.
pub struct StringReplacement;

impl Cop for StringReplacement {
    fn name(&self) -> &'static str {
        "Performance/StringReplacement"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE]
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

        // RuboCop matches any receiver including nil (bare gsub call, implicit self)

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 2 {
            return;
        }

        let mut args_iter = args.iter();
        let first_node = match args_iter.next() {
            Some(a) => a,
            None => return,
        };
        let second_node = match args_iter.next() {
            Some(a) => a,
            None => return,
        };

        // First arg: string literal or deterministic regex literal
        let first_is_single_char = if let Some(first) = first_node.as_string_node() {
            let first_str = first.unescaped();
            let first_text = String::from_utf8_lossy(first_str);
            first_text.chars().count() == 1
        } else if let Some(regex) = first_node.as_regular_expression_node() {
            is_deterministic_single_char_regex(regex)
        } else {
            false
        };

        if !first_is_single_char {
            return;
        }

        // Second arg must be a string literal
        let second = match second_node.as_string_node() {
            Some(s) => s,
            None => return,
        };

        let second_str = second.unescaped();

        // Second arg must be empty or a single character
        let second_text = String::from_utf8_lossy(second_str);
        let second_char_count = second_text.chars().count();
        if second_char_count > 1 {
            return;
        }

        let (prefer, current) = if second_char_count == 0 {
            // Empty replacement → delete
            if is_bang {
                ("delete!", "gsub!")
            } else {
                ("delete", "gsub")
            }
        } else {
            // Single char replacement → tr
            if is_bang {
                ("tr!", "gsub!")
            } else {
                ("tr", "gsub")
            }
        };

        // RuboCop points at the method name through end of args (node.loc.selector → end)
        let loc = call.message_loc().unwrap_or_else(|| call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Use `{prefer}` instead of `{current}`."),
        ));
    }
}

/// Check if a regex node represents a deterministic single-character pattern.
/// RuboCop's DETERMINISTIC_REGEX rejects patterns containing metacharacters
/// (`.`, `*`, `+`, `?`, `[`, `]`, `(`, `)`, `{`, `}`, `|`, `^`, `$`)
/// and regex-specific escape sequences (`\d`, `\s`, `\w`, `\b`, `\A`, `\Z`, etc.).
/// Simple escapes like `\t`, `\n`, `\r`, `\uXXXX`, `\xHH` are fine — they produce literal chars.
fn is_deterministic_single_char_regex(regex: ruby_prism::RegularExpressionNode<'_>) -> bool {
    // No flags allowed (e.g., /a/i)
    let closing = regex.closing_loc().as_slice();
    if closing.len() > 1 {
        return false;
    }

    let content = regex.content_loc().as_slice();

    // Empty regex is not a single char
    if content.is_empty() {
        return false;
    }

    // Check source content for regex metacharacters
    // Walk the raw source bytes to check for metacharacters and classify escapes
    let mut i = 0;
    let mut char_count = 0;
    while i < content.len() {
        if char_count > 1 {
            return false;
        }
        let b = content[i];
        match b {
            // Regex metacharacters — non-deterministic
            b'.' | b'*' | b'+' | b'?' | b'[' | b']' | b'(' | b')' | b'{' | b'}' | b'|' | b'^'
            | b'$' => return false,
            b'\\' => {
                // Escape sequence
                if i + 1 >= content.len() {
                    return false;
                }
                let next = content[i + 1];
                match next {
                    // Regex-specific escape classes — non-deterministic.
                    // Must match RuboCop's LITERAL_REGEX exclusion: [^AbBdDgGhHkpPRwWXsSzZ0-9]
                    b'd'
                    | b'D'
                    | b's'
                    | b'S'
                    | b'w'
                    | b'W'
                    | b'b'
                    | b'B'
                    | b'A'
                    | b'Z'
                    | b'z'
                    | b'G'
                    | b'g'
                    | b'h'
                    | b'H'
                    | b'R'
                    | b'p'
                    | b'P'
                    | b'k'
                    | b'X'
                    | b'0'..=b'9' => return false,
                    // Unicode escape: \uXXXX — counts as one char.
                    // RuboCop's LITERAL_REGEX only accepts \\u[\da-fA-F]{4} (4-digit form),
                    // NOT the braced form \u{XXXX}.
                    b'u' => {
                        i += 2;
                        if i < content.len() && content[i] == b'{' {
                            // \u{XXXX} braced form — not accepted by RuboCop
                            return false;
                        }
                        // \uXXXX form — skip 4 hex digits
                        let end = std::cmp::min(i + 4, content.len());
                        i = end;
                        char_count += 1;
                        continue;
                    }
                    // Hex escape: \xHH — one char
                    b'x' => {
                        i += 2;
                        let end = std::cmp::min(i + 2, content.len());
                        i = end;
                        char_count += 1;
                        continue;
                    }
                    // Simple escapes: \t, \n, \r, \\, \y, etc. — one char each
                    _ => {
                        i += 2;
                        char_count += 1;
                        continue;
                    }
                }
            }
            _ => {
                // RuboCop's LITERAL_REGEX only allows specific ASCII characters:
                // [\w\s\-,"'!#%&<>=;:`~/]
                // Non-ASCII bytes (>= 0x80) are not in the whitelist.
                if b >= 0x80 {
                    return false;
                }
                if !(b.is_ascii_alphanumeric()
                    || b == b'_'
                    || b.is_ascii_whitespace()
                    || matches!(
                        b,
                        b'-' | b','
                            | b'"'
                            | b'\''
                            | b'!'
                            | b'#'
                            | b'%'
                            | b'&'
                            | b'<'
                            | b'>'
                            | b'='
                            | b';'
                            | b':'
                            | b'`'
                            | b'~'
                            | b'/'
                    ))
                {
                    return false;
                }
                i += 1;
                char_count += 1;
            }
        }
    }
    char_count == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(StringReplacement, "cops/performance/string_replacement");
}
