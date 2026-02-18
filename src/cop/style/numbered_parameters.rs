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

        // Look for numbered block parameters (_1, _2, etc.)
        // In Prism, these are represented as NumberedParametersNode in blocks
        // Actually, in Prism, blocks with numbered parameters use `it` or `_1.._9` syntax
        // and the block node has a `numbered_parameters` field.

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

        // Check if block uses numbered parameters (no explicit parameters but uses _1 etc.)
        // In Prism, a block with numbered params has `parameters` as None but
        // the body references NumberedReferenceReadNode or similar.
        // Actually, numbered parameters (_1, _2, etc.) show up as LocalVariableReadNode
        // with names like "_1", "_2", etc.
        // We check if the block has no explicit parameters and its body uses _1.._9
        if block_node.parameters().is_some() {
            return Vec::new();
        }

        // Check body for _1.._9 usage
        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let body_src = std::str::from_utf8(body.location().as_slice()).unwrap_or("");
        let has_numbered = (1..=9).any(|i| {
            let param = format!("_{i}");
            body_src.contains(&param)
        });

        if !has_numbered {
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
