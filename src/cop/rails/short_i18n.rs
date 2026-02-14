use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct ShortI18n;

impl Cop for ShortI18n {
    fn name(&self) -> &'static str {
        "Rails/ShortI18n"
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
        let message = if method_name == b"translate" {
            "Use `I18n.t` instead of `I18n.translate`."
        } else if method_name == b"localize" {
            "Use `I18n.l` instead of `I18n.localize`."
        } else {
            return Vec::new();
        };

        // Receiver must be I18n
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

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, message.to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ShortI18n, "cops/rails/short_i18n");
}
