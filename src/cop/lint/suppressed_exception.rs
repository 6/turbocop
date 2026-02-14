use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct SuppressedException;

impl Cop for SuppressedException {
    fn name(&self) -> &'static str {
        "Lint/SuppressedException"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // RescueNode is visited via visit_begin_node's specific method,
        // not via the generic visit() dispatch. So we match BeginNode
        // and check its rescue_clause.
        let begin_node = match node.as_begin_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let rescue_node = match begin_node.rescue_clause() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let body_empty = match rescue_node.statements() {
            None => true,
            Some(stmts) => stmts.body().is_empty(),
        };

        if !body_empty {
            return Vec::new();
        }

        let kw_loc = rescue_node.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Do not suppress exceptions.".to_string(),
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
            &SuppressedException,
            include_bytes!("../../../testdata/cops/lint/suppressed_exception/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &SuppressedException,
            include_bytes!("../../../testdata/cops/lint/suppressed_exception/no_offense.rb"),
        );
    }
}
