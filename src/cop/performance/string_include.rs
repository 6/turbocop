use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct StringInclude;

/// Check if a regex pattern (raw content between slashes) contains no
/// regex metacharacters, meaning it's a simple literal string.
fn is_literal_regex(content: &[u8]) -> bool {
    if content.is_empty() {
        return false;
    }
    for &b in content {
        match b {
            // Regex metacharacters
            b'.' | b'*' | b'+' | b'?' | b'|' | b'(' | b')' | b'[' | b']' | b'{' | b'}'
            | b'^' | b'$' | b'\\' => return false,
            _ => {}
        }
    }
    true
}

/// Check if a node is a regex literal with no flags and a literal-only pattern.
fn is_simple_regex_node(node: &ruby_prism::Node<'_>) -> bool {
    let regex_node = match node.as_regular_expression_node() {
        Some(r) => r,
        None => return false,
    };
    // Skip if regex has flags (e.g., /pattern/i)
    let closing = regex_node.closing_loc().as_slice();
    if closing.len() > 1 {
        return false;
    }
    let content = regex_node.content_loc().as_slice();
    is_literal_regex(content)
}

impl Cop for StringInclude {
    fn name(&self) -> &'static str {
        "Performance/StringInclude"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();

        let is_match = match name {
            // str.match?(/regex/) or /regex/.match?(str) or str.match(/regex/) or /regex/.match(str)
            b"match?" | b"match" => {
                if call.receiver().is_none() {
                    return Vec::new();
                }
                let arguments = match call.arguments() {
                    Some(a) => a,
                    None => return Vec::new(),
                };
                let first_arg = match arguments.arguments().iter().next() {
                    Some(a) => a,
                    None => return Vec::new(),
                };
                let recv = call.receiver().unwrap();

                // Either the argument or the receiver must be a simple regex
                is_simple_regex_node(&first_arg) || is_simple_regex_node(&recv)
            }

            // /regex/ === str
            b"===" => {
                let recv = match call.receiver() {
                    Some(r) => r,
                    None => return Vec::new(),
                };
                is_simple_regex_node(&recv)
            }

            // str =~ /regex/ or /regex/ =~ str or !~
            b"=~" | b"!~" => {
                let recv = match call.receiver() {
                    Some(r) => r,
                    None => return Vec::new(),
                };
                let arguments = match call.arguments() {
                    Some(a) => a,
                    None => return Vec::new(),
                };
                let first_arg = match arguments.arguments().iter().next() {
                    Some(a) => a,
                    None => return Vec::new(),
                };
                is_simple_regex_node(&recv) || is_simple_regex_node(&first_arg)
            }

            _ => return Vec::new(),
        };

        if !is_match {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `String#include?` instead of a regex match with literal-only pattern.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(StringInclude, "cops/performance/string_include");
}
