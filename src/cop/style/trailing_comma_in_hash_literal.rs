use crate::cop::util::has_trailing_comma;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct TrailingCommaInHashLiteral;

impl Cop for TrailingCommaInHashLiteral {
    fn name(&self) -> &'static str {
        "Style/TrailingCommaInHashLiteral"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let hash_node = match node.as_hash_node() {
            Some(h) => h,
            None => return Vec::new(),
        };

        let closing_loc = hash_node.closing_loc();
        let elements = hash_node.elements();
        let last_elem = match elements.last() {
            Some(e) => e,
            None => return Vec::new(),
        };

        let last_end = last_elem.location().end_offset();
        let closing_start = closing_loc.start_offset();
        let bytes = source.as_bytes();

        if has_trailing_comma(bytes, last_end, closing_start) {
            let search_range = &bytes[last_end..closing_start];
            if let Some(comma_offset) = search_range.iter().position(|&b| b == b',') {
                let abs_offset = last_end + comma_offset;
                let (line, column) = source.offset_to_line_col(abs_offset);
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Avoid comma after the last item of a hash.".to_string(),
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
            &TrailingCommaInHashLiteral,
            include_bytes!(
                "../../../testdata/cops/style/trailing_comma_in_hash_literal/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &TrailingCommaInHashLiteral,
            include_bytes!(
                "../../../testdata/cops/style/trailing_comma_in_hash_literal/no_offense.rb"
            ),
        );
    }
}
