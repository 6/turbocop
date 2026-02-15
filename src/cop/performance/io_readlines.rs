// constant_name handles both as_constant_read_node and as_constant_path_node (qualified constants)
use crate::cop::util::{as_method_chain, constant_name};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct IoReadlines;

impl Cop for IoReadlines {
    fn name(&self) -> &'static str {
        "Performance/IoReadlines"
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
        let chain = match as_method_chain(node) {
            Some(c) => c,
            None => return Vec::new(),
        };

        if chain.inner_method != b"readlines" {
            return Vec::new();
        }

        if chain.outer_method != b"each" && chain.outer_method != b"map" {
            return Vec::new();
        }

        // Check that the inner call's receiver is IO or File
        let receiver = match chain.inner_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let class_name = match constant_name(&receiver) {
            Some(n) => n,
            None => return Vec::new(),
        };
        if class_name != b"IO" && class_name != b"File" {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, "Use `IO.foreach` instead of `IO.readlines.each`.".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(IoReadlines, "cops/performance/io_readlines");
}
