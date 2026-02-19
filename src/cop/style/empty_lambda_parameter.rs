use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_PARAMETERS_NODE, LAMBDA_NODE};

pub struct EmptyLambdaParameter;

impl Cop for EmptyLambdaParameter {
    fn name(&self) -> &'static str {
        "Style/EmptyLambdaParameter"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_PARAMETERS_NODE, LAMBDA_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check LambdaNode for empty parameters: -> () {}
        let lambda_node = match node.as_lambda_node() {
            Some(l) => l,
            None => return Vec::new(),
        };

        // Check if operator is -> (stabby lambda)
        let operator_loc = lambda_node.operator_loc();
        if operator_loc.as_slice() != b"->" {
            return Vec::new();
        }

        // Check parameters
        let params = match lambda_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        // For lambdas, parameters can be a BlockParametersNode
        // -> () {} would have a BlockParametersNode with opening_loc "(" and empty params
        let bp = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return Vec::new(),
        };

        // Must have parentheses as opening/closing
        let opening_loc = match bp.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        if opening_loc.as_slice() != b"(" {
            return Vec::new();
        }

        // Parameters must be empty
        if let Some(inner_params) = bp.parameters() {
            let has_params = !inner_params.requireds().is_empty()
                || !inner_params.optionals().is_empty()
                || inner_params.rest().is_some()
                || !inner_params.posts().is_empty()
                || !inner_params.keywords().is_empty()
                || inner_params.keyword_rest().is_some()
                || inner_params.block().is_some();
            if has_params {
                return Vec::new();
            }
        }

        let (line, column) = source.offset_to_line_col(opening_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Omit parentheses for the empty lambda parameters.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLambdaParameter, "cops/style/empty_lambda_parameter");
}
