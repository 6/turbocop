use crate::cop::node_type::REGULAR_EXPRESSION_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

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
        let content_str = match std::str::from_utf8(&content) {
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

        if has_mixed_captures(content_str) {
            let loc = regexp.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Do not mix named captures and numbered captures in a Regexp literal.".to_string(),
            ));
            return;
        }
    }
}

/// Check if a regexp pattern has both named and numbered (unnamed) capture groups.
fn has_mixed_captures(pattern: &str) -> bool {
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
