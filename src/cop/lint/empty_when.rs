use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyWhen;

impl Cop for EmptyWhen {
    fn name(&self) -> &'static str {
        "Lint/EmptyWhen"
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
        let when_node = match node.as_when_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let body_empty = match when_node.statements() {
            None => true,
            Some(stmts) => stmts.body().is_empty(),
        };

        if !body_empty {
            return Vec::new();
        }

        let kw_loc = when_node.keyword_loc();
        let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
        vec![Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message: "Avoid empty `when` conditions.".to_string(),
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
            &EmptyWhen,
            include_bytes!("../../../testdata/cops/lint/empty_when/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &EmptyWhen,
            include_bytes!("../../../testdata/cops/lint/empty_when/no_offense.rb"),
        );
    }
}
