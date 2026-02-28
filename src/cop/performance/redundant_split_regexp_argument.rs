use crate::cop::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantSplitRegexpArgument;

/// Check if regex content is a simple literal that could be replaced by a string.
/// Returns false for patterns with special regex characters like character classes,
/// quantifiers, alternation, anchors, etc.
///
/// Handles escape sequences: `\.` (escaped metachar) is a simple literal (just `.`),
/// `\n`, `\t`, `\r` are simple literals (newline/tab/CR), and `\\` is a literal
/// backslash. But `\s`, `\d`, `\w`, `\b`, `\A`, `\Z`, `\p`, `\h` etc. are true
/// regex features and are NOT simple literals.
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
            match next {
                // Escaped regex metacharacters — the literal character itself
                b'.' | b'*' | b'+' | b'?' | b'|' | b'(' | b')' | b'[' | b']' | b'{' | b'}'
                | b'^' | b'$' | b'\\' | b'#' | b'/' | b'-' => {
                    i += 2;
                }
                // Simple escape sequences that produce a single literal character
                b'n' | b't' | b'r' | b'f' | b'a' | b'e' | b'v' => {
                    i += 2;
                }
                // True regex features — NOT simple literals
                // \s \S \d \D \w \W \b \B \A \Z \z \G \p \P \h \H \R \X etc.
                _ => return false,
            }
        } else {
            match b {
                // Unescaped regex metacharacters — this is a real regex pattern
                b'.' | b'*' | b'+' | b'?' | b'|' | b'(' | b')' | b'[' | b']' | b'{' | b'}'
                | b'^' | b'$' | b'#' => return false,
                _ => {
                    i += 1;
                }
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

        let content = regex_node.content_loc().as_slice();
        if !is_simple_literal_regex(content) {
            return;
        }

        let loc = call.location();
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
