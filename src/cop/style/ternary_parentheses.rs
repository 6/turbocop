use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct TernaryParentheses;

impl Cop for TernaryParentheses {
    fn name(&self) -> &'static str {
        "Style/TernaryParentheses"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let if_node = match node.as_if_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        // Ternary has no if_keyword_loc
        if if_node.if_keyword_loc().is_some() {
            return Vec::new();
        }

        // Check if condition is wrapped in parentheses
        if let Some(paren) = if_node.predicate().as_parentheses_node() {
            let open_loc = paren.opening_loc();
            let (line, column) = source.offset_to_line_col(open_loc.start_offset());
            return vec![Diagnostic {
                path: source.path_str().to_string(),
                location: Location { line, column },
                severity: Severity::Convention,
                cop_name: self.name().to_string(),
                message: "Ternary conditions should not be wrapped in parentheses.".to_string(),
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
            &TernaryParentheses,
            include_bytes!("../../../testdata/cops/style/ternary_parentheses/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &TernaryParentheses,
            include_bytes!("../../../testdata/cops/style/ternary_parentheses/no_offense.rb"),
        );
    }
}
