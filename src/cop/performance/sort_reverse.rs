use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct SortReverse;

impl Cop for SortReverse {
    fn name(&self) -> &'static str {
        "Performance/SortReverse"
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

        if chain.inner_method != b"sort" || chain.outer_method != b"reverse" {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use `sort` with a block and reverse the comparison instead of `sort.reverse`."
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
            &SortReverse,
            include_bytes!("../../../testdata/cops/performance/sort_reverse/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &SortReverse,
            include_bytes!("../../../testdata/cops/performance/sort_reverse/no_offense.rb"),
        );
    }
}
