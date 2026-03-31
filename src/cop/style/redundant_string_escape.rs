use crate::cop::node_type::{INTERPOLATED_STRING_NODE, STRING_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;

/// Checks for redundant escape sequences in string literals.
///
/// Handles double-quoted strings (`"..."`), interpolating heredocs,
/// interpolating percent literals (`%(...)`, `%Q(...)`), and `%W[...]`
/// array elements that Prism represents as child string nodes without their
/// own delimiter metadata.
///
/// The current fix set closes the real Prism FNs we could reproduce against
/// RuboCop:
/// - `%W[\" ...]` child strings now inherit `%W` delimiter rules from the
///   parent array, so escaped quotes are checked instead of skipped.
/// - Escaped Unicode punctuation like `\“` and `\”` is now flagged while
///   escaped Unicode letters like `\ê` remain allowed, matching RuboCop's
///   Unicode-aware alnum exemption.
///
/// We re-checked the corpus-reported heredoc `\ ` examples against RuboCop
/// 1.84.2 with `PARSER_ENGINE=parser_prism`, and they did not reproduce, so
/// the heredoc/`%W` escaped-space exemption stays narrow to avoid regressing
/// existing matches.
pub struct RedundantStringEscape;

/// Escape sequences that are always meaningful in double-quoted-style strings.
/// This includes \\, standard escape letters, octal digits, \x, \u, \c, \C, \M,
/// and literal newline/carriage-return after backslash (line continuation).
/// Note: \", \', and \# are NOT here — they require context-dependent checks.
const MEANINGFUL_ESCAPES: &[u8] = b"\\abefnrstv01234567xucCM\n\r";

/// Returns the matching closing bracket for an opening bracket byte,
/// or the same byte for symmetric delimiters.
fn matching_bracket(open: u8) -> u8 {
    match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        other => other,
    }
}

#[derive(Clone)]
struct StringContext {
    delimiter_chars: Vec<u8>,
    allow_escaped_space: bool,
}

/// Analyze the opening delimiter to determine if the string supports
/// escape processing. Returns delimiter context, or None if the string type
/// should not be processed (single-quoted, %q/%w/%i, etc.).
fn analyze_opening(open_bytes: &[u8]) -> Option<StringContext> {
    // Standard double-quoted string
    if open_bytes == b"\"" {
        return Some(StringContext {
            delimiter_chars: vec![b'"'],
            allow_escaped_space: false,
        });
    }

    // Heredocs: <<FOO, <<-FOO, <<~FOO, <<"FOO", <<-"FOO", <<~"FOO"
    // Skip single-quoted heredocs: <<'FOO', <<-'FOO', <<~'FOO'
    if open_bytes.starts_with(b"<<") {
        if open_bytes.contains(&b'\'') {
            return None;
        }
        return Some(StringContext {
            delimiter_chars: vec![],
            allow_escaped_space: true,
        });
    }

    // Percent literals: %(foo), %Q(foo), %Q!foo!, %W[foo], etc.
    // Skip non-interpolating: %q, %w, %i
    if open_bytes.starts_with(b"%") && open_bytes.len() >= 2 {
        let second = open_bytes[1];
        if second == b'q' || second == b'w' || second == b'i' {
            return None;
        }
        let last = *open_bytes.last()?;
        let closing = matching_bracket(last);
        let mut delimiters = vec![last];
        if closing != last {
            delimiters.push(closing);
        }
        return Some(StringContext {
            delimiter_chars: delimiters,
            allow_escaped_space: second == b'W',
        });
    }

    None
}

fn inherited_array_context(
    parse_result: &ruby_prism::ParseResult<'_>,
    target: &ruby_prism::Node<'_>,
) -> Option<StringContext> {
    struct Finder {
        target_start: usize,
        target_end: usize,
        target_is_interpolated: bool,
        array_stack: Vec<StringContext>,
        array_marks: Vec<bool>,
        result: Option<StringContext>,
    }

    impl Finder {
        fn matches_target(&self, node: &ruby_prism::Node<'_>) -> bool {
            node.location().start_offset() == self.target_start
                && node.location().end_offset() == self.target_end
                && (node.as_interpolated_string_node().is_some() == self.target_is_interpolated)
                && (node.as_string_node().is_some() != self.target_is_interpolated)
        }
    }

    impl<'pr> Visit<'pr> for Finder {
        fn visit_branch_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
            let pushed = if let Some(array) = node.as_array_node() {
                array
                    .opening_loc()
                    .and_then(|opening| analyze_opening(opening.as_slice()))
                    .inspect(|ctx| self.array_stack.push(ctx.clone()))
                    .is_some()
            } else {
                false
            };
            self.array_marks.push(pushed);

            if self.matches_target(&node) {
                self.result = self.array_stack.last().cloned();
            }
        }

        fn visit_branch_node_leave(&mut self) {
            if self.array_marks.pop().unwrap_or(false) {
                self.array_stack.pop();
            }
        }

        fn visit_leaf_node_enter(&mut self, node: ruby_prism::Node<'pr>) {
            if self.matches_target(&node) {
                self.result = self.array_stack.last().cloned();
            }
        }
    }

    let mut finder = Finder {
        target_start: target.location().start_offset(),
        target_end: target.location().end_offset(),
        target_is_interpolated: target.as_interpolated_string_node().is_some(),
        array_stack: Vec::new(),
        array_marks: Vec::new(),
        result: None,
    };
    finder.visit(&parse_result.node());
    finder.result
}

impl RedundantStringEscape {
    fn escaped_char(content: &[u8], start: usize) -> (String, usize, Option<u8>, bool) {
        let byte = content[start];
        if byte.is_ascii() {
            let ch = byte as char;
            return (ch.to_string(), 1, Some(byte), ch.is_ascii_alphanumeric());
        }

        if let Ok(rest) = std::str::from_utf8(&content[start..]) {
            if let Some(ch) = rest.chars().next() {
                return (ch.to_string(), ch.len_utf8(), None, ch.is_alphanumeric());
            }
        }

        (
            String::from_utf8_lossy(&content[start..start + 1]).into_owned(),
            1,
            None,
            false,
        )
    }

    /// Scan raw string content bytes for redundant escape sequences.
    /// `content` is the raw source bytes between delimiters.
    /// `content_start` is the absolute byte offset of the start of content.
    /// `delimiter_chars` contains the chars that are valid to escape (delimiters).
    /// `allow_escaped_space` matches RuboCop's allowance for `\ ` in heredocs
    /// and `%W` array literals.
    fn scan_escapes(
        &self,
        source: &SourceFile,
        content: &[u8],
        content_start: usize,
        delimiter_chars: &[u8],
        allow_escaped_space: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut i = 0;

        while i < content.len() {
            if content[i] == b'\\' && i + 1 < content.len() {
                let (escaped_display, escaped_len, escaped_ascii, escaped_is_alnum) =
                    Self::escaped_char(content, i + 1);

                let is_redundant = if let Some(escaped) = escaped_ascii {
                    if MEANINGFUL_ESCAPES.contains(&escaped) {
                        false
                    } else if escaped.is_ascii_alphanumeric() {
                        // Alphanumeric escapes are never redundant (Ruby could give them
                        // meaning in future versions, and many already have meaning).
                        false
                    } else if delimiter_chars.contains(&escaped) {
                        // Escaping the delimiter character is meaningful
                        false
                    } else if escaped == b'#' {
                        // \# is only meaningful when disabling interpolation:
                        // \#{, \#$, \#@
                        if i + 2 < content.len() {
                            let next = content[i + 2];
                            if next == b'{' || next == b'$' || next == b'@' {
                                // \#{, \#$, \#@ — disabling interpolation
                                false
                            } else if next == b'\\'
                                && i + 3 < content.len()
                                && content[i + 3] == b'{'
                            {
                                // \#\{ — \# is not redundant (pairs with \{ to disable interp)
                                false
                            } else {
                                true
                            }
                        } else {
                            // \# at end of content — redundant
                            true
                        }
                    } else if escaped == b'{' || escaped == b'$' || escaped == b'@' {
                        // Check if preceded by '#' (not '\#') — disabling interpolation
                        // Patterns: #\{, #\$, #\@
                        if i > 0 && content[i - 1] == b'#' {
                            // Count consecutive backslashes before the '#'
                            let hash_pos = i - 1;
                            let mut bs_count: usize = 0;
                            let mut p = hash_pos;
                            while p > 0 {
                                p -= 1;
                                if content[p] == b'\\' {
                                    bs_count += 1;
                                } else {
                                    break;
                                }
                            }
                            // Even backslashes (including 0): '#' is literal → not redundant
                            // Odd backslashes: '#' is escaped (\#\{) → \{ is redundant
                            bs_count % 2 != 0
                        } else {
                            true
                        }
                    } else if escaped == b' ' && allow_escaped_space {
                        false
                    } else {
                        // Any other non-alphanumeric, non-meaningful escape is redundant
                        true
                    }
                } else if escaped_is_alnum {
                    // Alphanumeric escapes are never redundant (Ruby could give them
                    // meaning in future versions, and many already have meaning).
                    false
                } else {
                    // Unicode punctuation and other non-alnum chars are redundant.
                    true
                };

                if is_redundant {
                    let abs_offset = content_start + i;
                    let (line, column) = source.offset_to_line_col(abs_offset);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        column,
                        format!("Redundant escape of `{}` in string.", escaped_display),
                    ));
                }
                i += 1 + escaped_len;
            } else {
                i += 1;
            }
        }
    }
}

impl Cop for RedundantStringEscape {
    fn name(&self) -> &'static str {
        "Style/RedundantStringEscape"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[STRING_NODE, INTERPOLATED_STRING_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if let Some(s) = node.as_string_node() {
            let context = if let Some(opening_loc) = s.opening_loc() {
                match analyze_opening(opening_loc.as_slice()) {
                    Some(ctx) => ctx,
                    None => return,
                }
            } else {
                if !s.content_loc().as_slice().contains(&b'\\') {
                    return;
                }
                match inherited_array_context(parse_result, node) {
                    Some(ctx) => ctx,
                    None => return,
                }
            };

            let content_loc = s.content_loc();
            let content = content_loc.as_slice();
            let content_start = content_loc.start_offset();
            self.scan_escapes(
                source,
                content,
                content_start,
                &context.delimiter_chars,
                context.allow_escaped_space,
                diagnostics,
            );
        } else if let Some(s) = node.as_interpolated_string_node() {
            let context = if let Some(opening_loc) = s.opening_loc() {
                match analyze_opening(opening_loc.as_slice()) {
                    Some(ctx) => ctx,
                    None => return,
                }
            } else {
                match inherited_array_context(parse_result, node) {
                    Some(ctx) => ctx,
                    None => return,
                }
            };

            // Scan each string part within the interpolated string.
            // EmbeddedStatements parts (#{...}) are skipped — only string segments.
            for part in s.parts().iter() {
                if let Some(str_part) = part.as_string_node() {
                    let content_loc = str_part.content_loc();
                    let content = content_loc.as_slice();
                    let content_start = content_loc.start_offset();
                    self.scan_escapes(
                        source,
                        content,
                        content_start,
                        &context.delimiter_chars,
                        context.allow_escaped_space,
                        diagnostics,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantStringEscape, "cops/style/redundant_string_escape");
}
