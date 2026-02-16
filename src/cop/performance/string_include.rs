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

        if call.name().as_slice() != b"match?" {
            return Vec::new();
        }

        // Must have a receiver
        if call.receiver().is_none() {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        let first_arg = match args.iter().next() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let regex_node = match first_arg.as_regular_expression_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Skip if regex has flags (e.g., /pattern/i) — include? can't replicate flags
        let closing = regex_node.closing_loc().as_slice();
        if closing.len() > 1 {
            return Vec::new();
        }

        let content = regex_node.content_loc().as_slice();
        if !is_literal_regex(content) {
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
