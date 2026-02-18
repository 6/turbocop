use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct PercentLiteralDelimiters;

impl Cop for PercentLiteralDelimiters {
    fn name(&self) -> &'static str {
        "Style/PercentLiteralDelimiters"
    }

    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _preferred = config.get_string_hash("PreferredDelimiters");

        // Parse preferred delimiters from config
        // Default: () for most, [] for %i/%I/%w/%W, {} for %r
        let default_open = b'(';
        let default_close = b')';

        let src = source.as_bytes();
        let mut diagnostics = Vec::new();
        let mut i = 0;

        while i < src.len() {
            // Look for percent literals: %w, %W, %i, %I, %q, %Q, %r, %s, %x
            if src[i] == b'%' && i + 1 < src.len() {
                let next = src[i + 1];
                let is_percent_literal = matches!(
                    next,
                    b'w' | b'W' | b'i' | b'I' | b'q' | b'Q' | b'r' | b's' | b'x'
                );

                if is_percent_literal && i + 2 < src.len() {
                    let delimiter = src[i + 2];
                    let literal_type = format!("%{}", next as char);

                    // Determine expected delimiter for this type
                    let (expected_open, expected_close) = match next {
                        b'i' | b'I' | b'w' | b'W' => (b'[', b']'),
                        b'r' => (b'{', b'}'),
                        _ => (default_open, default_close),
                    };

                    if delimiter != expected_open {
                        let (line, column) = source.offset_to_line_col(i);
                        // Find the closing delimiter to get the full span
                        let close = Self::find_closing_delimiter(src, i + 2);
                        if close > i + 2 {
                            diagnostics.push(self.diagnostic(
                                source,
                                line,
                                column,
                                format!(
                                    "`{literal_type}`-literals should be delimited by `{}` and `{}`.",
                                    expected_open as char, expected_close as char,
                                ),
                            ));
                        }
                    }
                    // Skip past the literal
                    if i + 2 < src.len() {
                        let end = Self::find_closing_delimiter(src, i + 2);
                        i = end + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }

        diagnostics
    }
}

impl PercentLiteralDelimiters {
    fn find_closing_delimiter(src: &[u8], open_pos: usize) -> usize {
        if open_pos >= src.len() {
            return open_pos;
        }
        let open = src[open_pos];
        let close = match open {
            b'(' => b')',
            b'[' => b']',
            b'{' => b'}',
            b'<' => b'>',
            other => other, // symmetric delimiter like | or !
        };

        let mut depth = 1;
        let mut pos = open_pos + 1;
        while pos < src.len() && depth > 0 {
            if src[pos] == b'\\' {
                pos += 2;
                continue;
            }
            if open != close && src[pos] == open {
                depth += 1;
            } else if src[pos] == close {
                depth -= 1;
            }
            if depth > 0 {
                pos += 1;
            }
        }
        pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(PercentLiteralDelimiters, "cops/style/percent_literal_delimiters");
}
