use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct Count;

impl Cop for Count {
    fn name(&self) -> &'static str {
        "Performance/Count"
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

        if chain.outer_method != b"count" {
            return Vec::new();
        }

        let inner = chain.inner_method;
        let inner_name = if inner == b"select" {
            "select"
        } else if inner == b"reject" {
            "reject"
        } else {
            return Vec::new();
        };

        // The inner call should have a block
        if chain.inner_call.block().is_none() {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: format!("Use `count` instead of `{inner_name}...count`."),
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
            &Count,
            include_bytes!("../../../testdata/cops/performance/count/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &Count,
            include_bytes!("../../../testdata/cops/performance/count/no_offense.rb"),
        );
    }
}
