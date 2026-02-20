use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{INTERPOLATED_REGULAR_EXPRESSION_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE};

pub struct UnescapedBracketInRegexp;

impl Cop for UnescapedBracketInRegexp {
    fn name(&self) -> &'static str {
        "Lint/UnescapedBracketInRegexp"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[INTERPOLATED_REGULAR_EXPRESSION_NODE, REGULAR_EXPRESSION_NODE, STRING_NODE]
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
        // Check RegularExpressionNode
        if let Some(regexp) = node.as_regular_expression_node() {
            let content = regexp.unescaped();
            let content_str = match std::str::from_utf8(&content) {
                Ok(s) => s,
                Err(_) => return,
            };

            // Check for interpolation — skip
            let raw_src = &source.as_bytes()
                [regexp.location().start_offset()..regexp.location().end_offset()];
            if raw_src.windows(2).any(|w| w == b"#{") {
                return;
            }

            // The offset of the regexp content within the source (after the opening /)
            let content_start = regexp.content_loc().start_offset();

            diagnostics.extend(find_unescaped_brackets(self, source, content_str, content_start));
            return;
        }

        // Check InterpolatedRegularExpressionNode
        if let Some(interp_regexp) = node.as_interpolated_regular_expression_node() {
            for part in interp_regexp.parts().iter() {
                if let Some(s) = part.as_string_node() {
                    let content = s.unescaped();
                    let content_str = match std::str::from_utf8(&content) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    let content_start = s.content_loc().start_offset();
                    diagnostics.extend(find_unescaped_brackets(self, source, content_str, content_start));
                }
            }
            return;
        }

    }
}

/// Skip a character class starting at `bytes[pos]` == b'['.
/// Returns the position after the closing `]`.
/// Handles nested character classes `[a[b]]` and POSIX classes `[[:alpha:]]`.
fn skip_char_class(bytes: &[u8], start: usize) -> usize {
    let len = bytes.len();
    let mut i = start + 1; // past the opening [

    // Handle ^ (negation)
    if i < len && bytes[i] == b'^' {
        i += 1;
    }
    // `]` as first char in class is literal
    if i < len && bytes[i] == b']' {
        i += 1;
    }

    while i < len {
        if bytes[i] == b'\\' {
            i += 2; // skip escaped char
        } else if bytes[i] == b'[' {
            // Nested character class or POSIX class — recurse
            i = skip_char_class(bytes, i);
        } else if bytes[i] == b']' {
            return i + 1; // past the closing ]
        } else {
            i += 1;
        }
    }

    i // unterminated class — return end
}

fn find_unescaped_brackets(
    cop: &UnescapedBracketInRegexp,
    source: &SourceFile,
    content: &str,
    content_start: usize,
) -> Vec<Diagnostic> {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut diagnostics = Vec::new();
    let mut is_first_char = true;

    while i < len {
        if bytes[i] == b'\\' {
            i += 2; // skip escaped char
            is_first_char = false;
            continue;
        }

        // Skip character classes `[...]`, including nested classes and POSIX classes
        if bytes[i] == b'[' {
            is_first_char = false;
            i = skip_char_class(bytes, i);
            continue;
        }

        if bytes[i] == b']' {
            // `]` as the very first character of the regexp is not an offense
            // (Ruby doesn't warn about it)
            if !is_first_char {
                let offset = content_start + i;
                let (line, column) = source.offset_to_line_col(offset);
                diagnostics.push(cop.diagnostic(
                    source,
                    line,
                    column,
                    "Regular expression has `]` without escape.".to_string(),
                ));
            }
        }

        is_first_char = false;
        i += 1;
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnescapedBracketInRegexp, "cops/lint/unescaped_bracket_in_regexp");
}
