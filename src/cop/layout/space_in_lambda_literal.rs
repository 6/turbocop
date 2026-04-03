use crate::cop::shared::node_type::LAMBDA_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// Corpus investigation (2026-03-16):
/// - 17 FPs fixed: all were `-> () {` (lambda with empty parenthesized params).
///   RuboCop's `arrow_lambda_with_args?` checks `node.parent.arguments?` which returns
///   false for empty param lists. Fix: check that BlockParametersNode contains actual
///   parameters (requireds/optionals/rest/keywords/block), not just empty parens.
/// - 627 FNs fixed (2026-03-17): cop only checked for parenthesized params (`-> (x) {`),
///   missing non-parenthesized cases (`-> x {`). Fix: when no `(` found between `->` and
///   block opener, fall back to the ParametersNode location start to detect space.
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

        // Must have parameters with actual arguments (not empty parens `-> () {}`)
        let has_real_params = match lambda.parameters() {
            Some(params_node) => {
                // params_node is a Node wrapping BlockParametersNode
                if let Some(block_params) = params_node.as_block_parameters_node() {
                    // Check if the inner ParametersNode exists and has any requireds/optionals/etc.
                    match block_params.parameters() {
                        Some(p) => {
                            !p.requireds().is_empty()
                                || !p.optionals().is_empty()
                                || p.rest().is_some()
                                || !p.posts().is_empty()
                                || !p.keywords().is_empty()
                                || p.keyword_rest().is_some()
                                || p.block().is_some()
                        }
                        None => false,
                    }
                } else {
                    params_node.as_numbered_parameters_node().is_some()
                }
            }
            None => false,
        };
        if !has_real_params {
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

        // Determine the start of parameters (paren or first param char)
        let param_start = match between.iter().position(|&b| b == b'(') {
            Some(offset) => arrow_end + offset,
            None => {
                // Non-parenthesized params: find the BlockParametersNode start
                // which is the location of the first param character
                match lambda.parameters() {
                    Some(params_node) => {
                        if let Some(block_params) = params_node.as_block_parameters_node() {
                            match block_params.parameters() {
                                Some(p) => p.location().start_offset(),
                                None => return,
                            }
                        } else {
                            return;
                        }
                    }
                    None => return,
                }
            }
        };

        let has_space = param_start > arrow_end
            && bytes[arrow_end..param_start]
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
