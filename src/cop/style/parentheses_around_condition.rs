use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::source::SourceFile;

pub struct ParenthesesAroundCondition;

impl Cop for ParenthesesAroundCondition {
    fn name(&self) -> &'static str {
        "Style/ParenthesesAroundCondition"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        if let Some(if_node) = node.as_if_node() {
            // Must have `if` keyword (not ternary)
            let kw_loc = match if_node.if_keyword_loc() {
                Some(loc) => loc,
                None => return Vec::new(),
            };

            if let Some(paren) = if_node.predicate().as_parentheses_node() {
                let keyword = if kw_loc.as_slice() == b"unless" {
                    "unless"
                } else {
                    "if"
                };
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: format!(
                        "Don't use parentheses around the condition of an `{keyword}`."
                    ),
                }];
            }
        } else if let Some(while_node) = node.as_while_node() {
            if let Some(paren) = while_node.predicate().as_parentheses_node() {
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Don't use parentheses around the condition of a `while`.".to_string(),
                }];
            }
        } else if let Some(until_node) = node.as_until_node() {
            if let Some(paren) = until_node.predicate().as_parentheses_node() {
                let open_loc = paren.opening_loc();
                let (line, column) = source.offset_to_line_col(open_loc.start_offset());
                return vec![Diagnostic {
                    path: source.path_str().to_string(),
                    location: Location { line, column },
                    severity: Severity::Convention,
                    cop_name: self.name().to_string(),
                    message: "Don't use parentheses around the condition of an `until`."
                        .to_string(),
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
            &ParenthesesAroundCondition,
            include_bytes!(
                "../../../testdata/cops/style/parentheses_around_condition/offense.rb"
            ),
        );
    }

    #[test]
    fn no_offense_fixture() {
        assert_cop_no_offenses_full(
            &ParenthesesAroundCondition,
            include_bytes!(
                "../../../testdata/cops/style/parentheses_around_condition/no_offense.rb"
            ),
        );
    }
}
