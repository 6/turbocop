use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct BigDecimalWithNumericArgument;

impl Cop for BigDecimalWithNumericArgument {
    fn name(&self) -> &'static str {
        "Performance/BigDecimalWithNumericArgument"
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

        if call.name().as_slice() != b"BigDecimal" {
            return Vec::new();
        }

        // BigDecimal() is a Kernel method, so no receiver
        if call.receiver().is_some() {
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

        // Check if first argument is an IntegerNode or FloatNode
        let first_arg = match args.iter().next() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let is_numeric = matches!(
            first_arg,
            ruby_prism::Node::IntegerNode { .. } | ruby_prism::Node::FloatNode { .. }
        );

        if !is_numeric {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use a string argument to `BigDecimal` instead of a numeric argument."
                .to_string(),
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
            &BigDecimalWithNumericArgument,
            include_bytes!(
                "../../../testdata/cops/performance/big_decimal_with_numeric_argument/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &BigDecimalWithNumericArgument,
            include_bytes!(
                "../../../testdata/cops/performance/big_decimal_with_numeric_argument/no_offense.rb"
            ),
        );
    }
}
