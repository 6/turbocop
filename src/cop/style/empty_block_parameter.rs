use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE};

pub struct EmptyBlockParameter;

impl Cop for EmptyBlockParameter {
    fn name(&self) -> &'static str {
        "Style/EmptyBlockParameter"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        // Check BlockNode for empty parameters (||)
        let block_node = match node.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let params = match block_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let bp = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return Vec::new(),
        };

        // Must have pipe delimiters (opening_loc and closing_loc)
        let opening_loc = match bp.opening_loc() {
            Some(loc) => loc,
            None => return Vec::new(),
        };

        if opening_loc.as_slice() != b"|" {
            return Vec::new();
        }

        // Parameters must be empty (no actual params)
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

        // Locals must be empty too (no block-local vars like `do |; x|`)
        if !bp.locals().is_empty() {
            return Vec::new();
        }

        let (line, column) = source.offset_to_line_col(opening_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Omit pipes for the empty block parameters.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyBlockParameter, "cops/style/empty_block_parameter");
}
