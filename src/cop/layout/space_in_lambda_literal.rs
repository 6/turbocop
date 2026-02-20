use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::LAMBDA_NODE;

pub struct SpaceInLambdaLiteral;

impl Cop for SpaceInLambdaLiteral {
    fn name(&self) -> &'static str {
        "Layout/SpaceInLambdaLiteral"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[LAMBDA_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "require_no_space");

        let lambda = match node.as_lambda_node() {
            Some(l) => l,
            None => return,
        };

        // Must have parameters
        if lambda.parameters().is_none() {
            return;
        }

        let operator_loc = lambda.operator_loc();
        let arrow_end = operator_loc.end_offset();
        let opening_loc = lambda.opening_loc();
        let opening_start = opening_loc.start_offset();

        let bytes = source.as_bytes();
        let search_end = opening_start.min(bytes.len());

        // Find the opening paren between -> and { or do
        let between = if arrow_end < search_end {
            &bytes[arrow_end..search_end]
        } else {
            return;
        };

        // Must have parenthesized parameters
        let paren_offset_in_between = between.iter().position(|&b| b == b'(');
        let paren_pos = match paren_offset_in_between {
            Some(offset) => arrow_end + offset,
            None => return,
        };

        let has_space = paren_pos > arrow_end
            && bytes[arrow_end..paren_pos]
                .iter()
                .any(|&b| b == b' ' || b == b'\t');

        match style {
            "require_space" => {
                if !has_space {
                    let (line, col) = source.offset_to_line_col(arrow_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Use a space between `->` and `(` in lambda literals.".to_string(),
                    ));
                }
            }
            _ => {
                // "require_no_space" (default)
                if has_space {
                    let (line, col) = source.offset_to_line_col(arrow_end);
                    diagnostics.push(self.diagnostic(
                        source,
                        line,
                        col,
                        "Do not use spaces between `->` and `(` in lambda literals.".to_string(),
                    ));
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(SpaceInLambdaLiteral, "cops/layout/space_in_lambda_literal");
}
