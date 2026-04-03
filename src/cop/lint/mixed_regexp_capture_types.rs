use crate::cop::shared::node_type::REGULAR_EXPRESSION_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-07):
/// Root cause of 12 FPs: two bugs in the hand-rolled `has_mixed_captures()` parser.
/// 1. Conditional backreferences `(?(<name>)...)` / `(?('name')...)` — the inner
///    `(<name>)` was falsely counted as a numbered capture group (4 FPs: jruby, natalie).
/// 2. Extended-mode (`/x`) comments containing parentheses — `# comment (example)`
///    was parsed as containing a numbered capture (8 FPs: dependabot, kamal, dotenv,
///    huginn, ruby-git, rdoc, roadie).
///
/// Fix: skip conditional backreference conditions after `(?(`, and strip `#`-comments
/// in extended mode before scanning.
pub struct MixedRegexpCaptureTypes;

impl Cop for MixedRegexpCaptureTypes {
    fn name(&self) -> &'static str {
        "Lint/MixedRegexpCaptureTypes"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        // Check RegularExpressionNode for mixed capture types
        let regexp = match node.as_regular_expression_node() {
            Some(r) => r,
            None => return,
        };

        // Get the regexp content (unescaped source between delimiters)
        let content = regexp.unescaped();
        let content_str = match std::str::from_utf8(content) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Skip regexps with interpolation (they have EmbeddedStatementsNode children)
        // We check the raw source for `#{` to detect interpolation
        let raw_src =
            &source.as_bytes()[regexp.location().start_offset()..regexp.location().end_offset()];
        if raw_src.windows(2).any(|w| w == b"#{") {
            return;
        }

        // Check for /x (extended) flag — comments may contain parens
        let closing = regexp.closing_loc().as_slice();
        let extended = closing.contains(&b'x');

        if has_mixed_captures(content_str, extended) {
            let loc = regexp.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not mix named captures and numbered captures in a Regexp literal.".to_string(),
            ));
        }
    }
}

/// Check if a regexp pattern has both named and numbered (unnamed) capture groups.
/// When `extended` is true (the `/x` flag), `#` starts a comment to end of line.
fn has_mixed_captures(pattern: &str, extended: bool) -> bool {
    let mut has_named = false;
    let mut has_numbered = false;

    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\\' {
            // Skip escaped characters
            i += 2;
            continue;
        }

        // In extended mode, `#` starts a comment to end of line
        if extended && bytes[i] == b'#' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Skip character classes `[...]` — parentheses inside are literal
        if bytes[i] == b'[' {
            i += 1;
            // `]` as the first char in a class is literal, e.g. `[]foo]`
            // Also handle `[^]...]`
            if i < len && bytes[i] == b'^' {
                i += 1;
            }
            if i < len && bytes[i] == b']' {
                i += 1;
            }
            while i < len && bytes[i] != b']' {
                if bytes[i] == b'\\' {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            // Skip the closing `]`
            if i < len {
                i += 1;
            }
            continue;
        }

        if bytes[i] == b'(' && i + 1 < len {
            if bytes[i + 1] == b'?' {
                // Look at what follows `(?`
                if i + 2 < len {
                    match bytes[i + 2] {
                        b'(' => {
                            // Conditional backreference `(?(name)...)` or `(?(<name>)...)`
                            // or `(?('name')...)` — skip the condition part entirely.
                            // The condition ends at the next `)`.
                            i += 3;
                            while i < len && bytes[i] != b')' {
                                i += 1;
                            }
                            // Skip the closing `)` of the condition
                            if i < len {
                                i += 1;
                            }
                            continue;
                        }
                        b'<' => {
                            // Could be named capture `(?<name>...)` or lookbehind `(?<=...)` / `(?<!...)`
                            if i + 3 < len && bytes[i + 3] != b'=' && bytes[i + 3] != b'!' {
                                has_named = true;
                            }
                            // lookbehind is not a capture at all, skip
                        }
                        b'\'' => {
                            // Named capture with single quotes: (?'name'...)
                            has_named = true;
                        }
                        b':' | b'=' | b'!' | b'>' | b'#' => {
                            // Non-capturing group (?:...), lookahead (?=...), (?!...),
                            // atomic (?>...), comment (?#...) — not captures
                        }
                        _ => {
                            // Other patterns like (?flags:...) — not captures
                        }
                    }
                }
            } else {
                // Plain `(...)` — numbered capture
                has_numbered = true;
            }
        }

        i += 1;
    }

    has_named && has_numbered
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        MixedRegexpCaptureTypes,
        "cops/lint/mixed_regexp_capture_types"
    );
}
