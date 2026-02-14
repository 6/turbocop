use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct ArraySemiInfiniteRangeSlice;

impl Cop for ArraySemiInfiniteRangeSlice {
    fn name(&self) -> &'static str {
        "Performance/ArraySemiInfiniteRangeSlice"
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

        if call.name().as_slice() != b"[]" {
            return Vec::new();
        }

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let args = arguments.arguments();
        if args.len() != 1 {
            return Vec::new();
        }

        let first_arg = args.iter().next().unwrap();
        let range = match first_arg.as_range_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Semi-infinite range: has a left but no right (e.g., 2..)
        if range.left().is_none() || range.right().is_some() {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use `drop` instead of `[]` with a semi-infinite range.".to_string(),
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
            &ArraySemiInfiniteRangeSlice,
            include_bytes!(
                "../../../testdata/cops/performance/array_semi_infinite_range_slice/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &ArraySemiInfiniteRangeSlice,
            include_bytes!(
                "../../../testdata/cops/performance/array_semi_infinite_range_slice/no_offense.rb"
            ),
        );
    }
}
