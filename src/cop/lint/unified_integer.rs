// Handles both as_constant_read_node and as_constant_path_node (qualified constants like ::Fixnum)
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CONSTANT_PATH_NODE, CONSTANT_READ_NODE};

pub struct UnifiedInteger;

impl Cop for UnifiedInteger {
    fn name(&self) -> &'static str {
        "Lint/UnifiedInteger"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let name = match constant_name(node) {
            Some(n) => n,
            None => return Vec::new(),
        };

        let message = if name == b"Fixnum" {
            "Use `Integer` instead of `Fixnum`."
        } else if name == b"Bignum" {
            "Use `Integer` instead of `Bignum`."
        } else {
            return Vec::new();
        };

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, message.to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UnifiedInteger, "cops/lint/unified_integer");
}
