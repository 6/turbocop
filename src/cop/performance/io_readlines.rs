use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
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

        let const_node = match receiver.as_constant_read_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let class_name = const_node.name().as_slice();
        if class_name != b"IO" && class_name != b"File" {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use `IO.foreach` instead of `IO.readlines.each`.".to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &IoReadlines,
            include_bytes!("../../../testdata/cops/performance/io_readlines/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &IoReadlines,
            include_bytes!("../../../testdata/cops/performance/io_readlines/no_offense.rb"),
        );
    }
}
