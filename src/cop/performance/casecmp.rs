use crate::cop::util::as_method_chain;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct Casecmp;

impl Cop for Casecmp {
    fn name(&self) -> &'static str {
        "Performance/Casecmp"
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

        if chain.outer_method != b"==" {
            return Vec::new();
        }

        if chain.inner_method != b"downcase" && chain.inner_method != b"upcase" {
            return Vec::new();
        }

        // downcase/upcase should have no arguments
        if chain.inner_call.arguments().is_some() {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: format!(
                "Use `casecmp` instead of `{} ==`.",
                std::str::from_utf8(chain.inner_method).unwrap_or("downcase")
            ),
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
            &Casecmp,
            include_bytes!("../../../testdata/cops/performance/casecmp/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &Casecmp,
            include_bytes!("../../../testdata/cops/performance/casecmp/no_offense.rb"),
        );
    }
}
