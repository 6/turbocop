use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NumberedParameters;

impl Cop for NumberedParameters {
    fn name(&self) -> &'static str {
        "Style/NumberedParameters"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let style = config.get_str("EnforcedStyle", "allow_single_line");

        // Check for blocks that use numbered parameters
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // In Prism, blocks with numbered params have parameters() set to a
        // NumberedParametersNode. Blocks with explicit params have BlockParametersNode.
        // Only proceed if parameters is a NumberedParametersNode — this is the
        // authoritative way to detect numbered parameter usage via the AST,
        // avoiding false positives from string matching _1.._9 in comments,
        // strings, or variable names like _1_foo.
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        if params.as_numbered_parameters_node().is_none() {
            return Vec::new();
        }

        if style == "disallow" {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Avoid using numbered parameters.".to_string(),
            )];
        }

        if style == "allow_single_line" {
            // Flag if multi-line block
            let block_loc = block_node.location();
            let (start_line, _) = source.offset_to_line_col(block_loc.start_offset());
            let (end_line, _) = source.offset_to_line_col(block_loc.end_offset().saturating_sub(1));
            if start_line != end_line {
                let loc = node.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                return vec![self.diagnostic(
                    source,
                    line,
                    column,
                    "Avoid using numbered parameters for multi-line blocks.".to_string(),
                )];
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NumberedParameters, "cops/style/numbered_parameters");
}
