use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantSplitRegexpArgument;

impl Cop for RedundantSplitRegexpArgument {
    fn name(&self) -> &'static str {
        "Performance/RedundantSplitRegexpArgument"
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

        if call.name().as_slice() != b"split" {
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
        if args.is_empty() {
            return Vec::new();
        }

        // Check if first argument is a RegularExpressionNode
        let first_arg = match args.iter().next() {
            Some(a) => a,
            None => return Vec::new(),
        };
        if first_arg.as_regular_expression_node().is_none() {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use string as argument instead of regexp.".to_string(),
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
            &RedundantSplitRegexpArgument,
            include_bytes!(
                "../../../testdata/cops/performance/redundant_split_regexp_argument/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &RedundantSplitRegexpArgument,
            include_bytes!(
                "../../../testdata/cops/performance/redundant_split_regexp_argument/no_offense.rb"
            ),
        );
    }
}
