use crate::cop::util::has_trailing_comma;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct TrailingCommaInArguments;

impl Cop for TrailingCommaInArguments {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInArguments"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let closing_loc = match call_node.closing_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        let arguments = match call_node.arguments() {
            Some(args) => args,
            None => return Vec::new(),
        };

        let arg_list = arguments.arguments();
        let last_arg = match arg_list.last() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let last_end = last_arg.location().end_offset();
        let closing_start = closing_loc.start_offset();
        let bytes = source.as_bytes();

        if has_trailing_comma(bytes, last_end, closing_start) {
            // Find the actual comma position
            let search_range = &bytes[last_end..closing_start];
            if let Some(comma_offset) = search_range.iter().position(|&b| b == b',') {
                let abs_offset = last_end + comma_offset;
                let (line, column) = source.offset_to_line_col(abs_offset);
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Avoid comma after the last parameter of a method call.".to_string(),
                }];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{assert_cop_no_offenses_full, assert_cop_offenses_full};

    #[test]
    fn offense_fixture() {
        assert_cop_offenses_full(
            &TrailingCommaInArguments,
            include_bytes!(
                "../../../testdata/cops/style/trailing_comma_in_arguments/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &TrailingCommaInArguments,
            include_bytes!(
                "../../../testdata/cops/style/trailing_comma_in_arguments/no_offense.rb"
            ),
        );
    }
}
