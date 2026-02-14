use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct NegatedIf;

impl Cop for NegatedIf {
    fn name(&self) -> &'static str {
        "Style/NegatedIf"
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

        // Must have an `if` keyword (not ternary)
        let if_kw_loc = match if_node.if_keyword_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        // Must actually be `if`, not `unless`
        if if_kw_loc.as_slice() != b"if" {
            return Vec::new();
        }

        // Must not have an else clause
        if if_node.subsequent().is_some() {
            return Vec::new();
        }

        // Check if predicate is a `!` call
        let predicate = if_node.predicate();
        if let Some(call) = predicate.as_call_node() {
            if call.name().as_slice() == b"!" {
                let (line, column) = source.offset_to_line_col(if_kw_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Favor `unless` over `if` for negative conditions.".to_string(),
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
            &NegatedIf,
            include_bytes!("../../../testdata/cops/style/negated_if/offense.rb"),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &NegatedIf,
            include_bytes!("../../../testdata/cops/style/negated_if/no_offense.rb"),
        );
    }
}
