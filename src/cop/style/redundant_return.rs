use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantReturn;

impl Cop for RedundantReturn {
    fn name(&self) -> &'static str {
        "Style/RedundantReturn"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        let body = match def_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let statements = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let last = match statements.body().last() {
            Some(n) => n,
            None => return Vec::new(),
        };

        if last.as_return_node().is_some() {
            let loc = last.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Redundant `return` detected.".to_string(),
            }];
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
            &RedundantReturn,
            include_bytes!("../../../testdata/cops/style/redundant_return/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &RedundantReturn,
            include_bytes!("../../../testdata/cops/style/redundant_return/no_offense.rb"),
        );
    }
}
