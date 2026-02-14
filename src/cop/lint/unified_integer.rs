use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct UnifiedInteger;

impl Cop for UnifiedInteger {
    fn name(&self) -> &'static str {
        "Lint/UnifiedInteger"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let const_node = match node.as_constant_read_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let name = const_node.name().as_slice();
        let message = if name == b"Fixnum" {
            "Use `Integer` instead of `Fixnum`."
        } else if name == b"Bignum" {
            "Use `Integer` instead of `Bignum`."
        } else {
            return Vec::new();
        };

        let loc = const_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, message.to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnifiedInteger, "cops/lint/unified_integer");
}
