use crate::cop::node_type::{CALL_NODE, REGULAR_EXPRESSION_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct StartWith;

/// Check if regex content starts with \A and the rest is a simple literal.
fn is_start_anchored_literal(content: &[u8], safe_multiline: bool) -> bool {
    if content.len() < 2 {
        return false;
    }
    // Check for \A anchor (always valid)
    if content.len() >= 3 && content[0] == b'\\' && content[1] == b'A' {
        let rest = &content[2..];
        if !rest.is_empty() && is_literal_chars(rest) {
            return true;
        }
    }
    // Check for ^ anchor (only when SafeMultiline is false)
    if !safe_multiline && content[0] == b'^' {
        let rest = &content[1..];
        if !rest.is_empty() && is_literal_chars(rest) {
            return true;
        }
    }
    false
}

/// Check if a byte is a "safe literal" character per RuboCop's LITERAL_REGEX:
/// `[\w\s\-,"'!#%&<>=;:\x60~/]` — word chars, whitespace, and specific punctuation.
/// Characters NOT in this set (like `@`, `(`, `.`, `*`, etc.) are not considered literal.
fn is_safe_literal_char(b: u8) -> bool {
    b.is_ascii_alphanumeric()
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
                | b'.'
        )
}

fn is_literal_chars(bytes: &[u8]) -> bool {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            // Escaped character: backslash + next char
            // RuboCop allows \\[^AbBdDgGhHkpPRwWXsSzZ0-9]
            if i + 1 >= bytes.len() {
                return false;
            }
            let next = bytes[i + 1];
            if matches!(
                next,
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
            ) || next.is_ascii_digit()
            {
                return false;
            }
            i += 2;
        } else if is_safe_literal_char(bytes[i]) {
            i += 1;
        } else {
            return false;
        }
    }
    true
}

/// Extract regex node from a Prism node, returning it if it's a RegularExpressionNode.
fn extract_regex_node<'a>(
    node: &'a ruby_prism::Node<'a>,
) -> Option<ruby_prism::RegularExpressionNode<'a>> {
    node.as_regular_expression_node()
}

/// Check if a regex node has no flags (ignore_case, extended, multi_line, once).
fn has_no_flags(regex: &ruby_prism::RegularExpressionNode<'_>) -> bool {
    !regex.is_ignore_case() && !regex.is_extended() && !regex.is_multi_line() && !regex.is_once()
}

impl Cop for StartWith {
    fn name(&self) -> &'static str {
        "Performance/StartWith"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let safe_multiline = config.get_bool("SafeMultiline", true);
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        match method_name {
            b"match?" | b"match" => {
                // Two orientations:
                // 1. str.match?(/\Aabc/) — receiver is string, arg is regex
                // 2. /\Aabc/.match?(str) — receiver is regex, arg is string
                if call.receiver().is_none() {
                    return;
                }
                let arguments = match call.arguments() {
                    Some(a) => a,
                    None => return,
                };
                let args = arguments.arguments();
                let first_arg = match args.iter().next() {
                    Some(a) => a,
                    None => return,
                };

                // Try arg as regex (str.match?(/regex/))
                let found = if let Some(regex_node) = extract_regex_node(&first_arg) {
                    if !has_no_flags(&regex_node) {
                        return;
                    }
                    let content = regex_node.content_loc().as_slice();
                    is_start_anchored_literal(content, safe_multiline)
                } else if let Some(recv) = call.receiver() {
                    // Try receiver as regex (/regex/.match?(str))
                    if let Some(regex_node) = extract_regex_node(&recv) {
                        if !has_no_flags(&regex_node) {
                            return;
                        }
                        let content = regex_node.content_loc().as_slice();
                        is_start_anchored_literal(content, safe_multiline)
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !found {
                    return;
                }
            }
            b"=~" => {
                // Two orientations:
                // 1. str =~ /\Aabc/ — receiver is string, arg is regex
                // 2. /\Aabc/ =~ str — receiver is regex, arg is string
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

                // Check if arg is the regex
                let found = if let Some(regex_node) = extract_regex_node(&first_arg) {
                    if !has_no_flags(&regex_node) {
                        return;
                    }
                    let content = regex_node.content_loc().as_slice();
                    is_start_anchored_literal(content, safe_multiline)
                } else if let Some(regex_node) = extract_regex_node(&recv) {
                    // Check if receiver is the regex
                    if !has_no_flags(&regex_node) {
                        return;
                    }
                    let content = regex_node.content_loc().as_slice();
                    is_start_anchored_literal(content, safe_multiline)
                } else {
                    false
                };

                if !found {
                    return;
                }
            }
            _ => return,
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `start_with?` instead of a regex match anchored to the beginning of the string."
                .to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(StartWith, "cops/performance/start_with");

    #[test]
    fn config_safe_multiline_false_flags_caret() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("SafeMultiline".into(), serde_yml::Value::Bool(false))]),
            ..CopConfig::default()
        };
        let source = b"'abc'.match?(/^ab/)\n";
        let diags = run_cop_full_with_config(&StartWith, source, config);
        assert!(
            !diags.is_empty(),
            "Should flag ^anchor when SafeMultiline:false"
        );
    }

    #[test]
    fn config_safe_multiline_true_ignores_caret() {
        use crate::testutil::run_cop_full_with_config;
        use std::collections::HashMap;

        let config = CopConfig {
            options: HashMap::from([("SafeMultiline".into(), serde_yml::Value::Bool(true))]),
            ..CopConfig::default()
        };
        let source = b"'abc'.match?(/^ab/)\n";
        let diags = run_cop_full_with_config(&StartWith, source, config);
        assert!(
            diags.is_empty(),
            "Should not flag ^anchor when SafeMultiline:true"
        );
    }
}
