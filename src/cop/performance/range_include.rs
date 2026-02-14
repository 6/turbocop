use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct RangeInclude;

impl Cop for RangeInclude {
    fn name(&self) -> &'static str {
        "Performance/RangeInclude"
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

        if call.name().as_slice() != b"include?" {
            return Vec::new();
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Check if receiver is a RangeNode directly or wrapped in parentheses
        let is_range = receiver.as_range_node().is_some()
            || receiver
                .as_parentheses_node()
                .and_then(|p| p.body())
                .and_then(|b| {
                    // The body of parentheses is a StatementsNode
                    let stmts = b.as_statements_node()?;
                    let body = stmts.body();
                    if body.len() == 1 {
                        body.iter().next()?.as_range_node()
                    } else {
                        None
                    }
                })
                .is_some();

        if !is_range {
            return Vec::new();
        }

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Use `Range#cover?` instead of `Range#include?`.".to_string(),
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
            &RangeInclude,
            include_bytes!("../../../testdata/cops/performance/range_include/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &RangeInclude,
            include_bytes!("../../../testdata/cops/performance/range_include/no_offense.rb"),
        );
    }
}
