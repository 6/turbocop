use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct NegatedWhile;

impl Cop for NegatedWhile {
    fn name(&self) -> &'static str {
        "Style/NegatedWhile"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let while_node = match node.as_while_node() {
            Some(n) => n,
            None => return Vec::new(),
        };

        let predicate = while_node.predicate();
        if let Some(call) = predicate.as_call_node() {
            if call.name().as_slice() == b"!" {
                let kw_loc = while_node.keyword_loc();
                let (line, column) = source.offset_to_line_col(kw_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Favor `until` over `while` for negative conditions.".to_string(),
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
            &NegatedWhile,
            include_bytes!("../../../testdata/cops/style/negated_while/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &NegatedWhile,
            include_bytes!("../../../testdata/cops/style/negated_while/no_offense.rb"),
        );
    }
}
