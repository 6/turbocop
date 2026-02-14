use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Squeeze;

impl Cop for Squeeze {
    fn name(&self) -> &'static str {
        "Performance/Squeeze"
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

        if call.name().as_slice() != b"gsub" {
            return Vec::new();
        }

        if call.receiver().is_none() {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        if args.len() != 2 {
            return Vec::new();
        }

        let mut iter = args.iter();
        let first_arg = iter.next().unwrap();
        let second_arg = iter.next().unwrap();

        // First arg must be a regex of the form X+ (2 bytes: a single char followed by +)
        let regex_node = match first_arg.as_regular_expression_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let regex_content = regex_node.content_loc().as_slice();
        // Pattern must be exactly 2 bytes: one literal char + '+'
        if regex_content.len() != 2 || regex_content[1] != b'+' {
            return Vec::new();
        }

        let repeat_char = regex_content[0];
        // The char must not be a metacharacter itself
        if matches!(
            repeat_char,
            b'.' | b'*' | b'+' | b'?' | b'|' | b'(' | b')' | b'[' | b']' | b'{' | b'}'
                | b'^' | b'$' | b'\\'
        ) {
            return Vec::new();
        }

        // Second arg must be a single-char string matching the same character
        let string_node = match second_arg.as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let replacement = string_node.unescaped();
        if replacement.len() != 1 || replacement[0] != repeat_char {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `squeeze` instead of `gsub`.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Squeeze, "cops/performance/squeeze");
}
