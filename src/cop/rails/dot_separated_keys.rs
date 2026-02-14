use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct DotSeparatedKeys;

impl Cop for DotSeparatedKeys {
    fn name(&self) -> &'static str {
        "Rails/DotSeparatedKeys"
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

        let method_name = call.name().as_slice();
        if method_name != b"t" && method_name != b"translate" {
            return Vec::new();
        }

        // Receiver should be I18n
        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };
        let const_read = match recv.as_constant_read_node() {
            Some(c) => c,
            None => return Vec::new(),
        };
        if const_read.name().as_slice() != b"I18n" {
            return Vec::new();
        }

        // First argument should be a string containing dots
        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        let string_node = match arg_list[0].as_string_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let content = string_node.unescaped();
        if !content.contains(&b'.') {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use symbol keys or scope option instead of dot-separated string keys.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DotSeparatedKeys, "cops/rails/dot_separated_keys");
}
