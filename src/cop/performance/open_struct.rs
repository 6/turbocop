// Handles both as_constant_read_node and as_constant_path_node (qualified constants like ::OpenStruct)
use crate::cop::util::constant_name;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct OpenStruct;

impl Cop for OpenStruct {
    fn name(&self) -> &'static str {
        "Performance/OpenStruct"
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
        let name = match constant_name(node) {
            Some(n) => n,
            None => return Vec::new(),
        };

        if name != b"OpenStruct" {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `Struct` instead of `OpenStruct`.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OpenStruct, "cops/performance/open_struct");
}
