use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct MethodObjectAsBlock;

impl Cop for MethodObjectAsBlock {
    fn name(&self) -> &'static str {
        "Performance/MethodObjectAsBlock"
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
        // Detect BlockArgumentNode whose expression is a call to `method`
        let block_arg = match node.as_block_argument_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let expr = match block_arg.expression() {
            Some(e) => e,
            None => return Vec::new(),
        };

        let call = match expr.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if call.name().as_slice() != b"method" {
            return Vec::new();
        }

        let loc = block_arg.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use a block instead of `&method(...)` for better performance.".to_string(),
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
            &MethodObjectAsBlock,
            include_bytes!(
                "../../../testdata/cops/performance/method_object_as_block/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &MethodObjectAsBlock,
            include_bytes!(
                "../../../testdata/cops/performance/method_object_as_block/no_offense.rb"
            ),
        );
    }
}
