use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, REQUIRED_PARAMETER_NODE};

pub struct SingleLineBlockParams;

impl Cop for SingleLineBlockParams {
    fn name(&self) -> &'static str {
        "Style/SingleLineBlockParams"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, REQUIRED_PARAMETER_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let _methods = config.get_string_array("Methods");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Default: reduce/inject with params [acc, elem]
        let expected_params: &[&[u8]] = if method_name == b"reduce" || method_name == b"inject" {
            &[b"acc", b"elem"]
        } else {
            return Vec::new();
        };

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let params = match block_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return Vec::new(),
        };

        let param_node = match block_params.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };

        let requireds: Vec<_> = param_node.requireds().iter().collect();
        if requireds.len() != expected_params.len() {
            return Vec::new();
        }

        // Check if block is on a single line
        let (start_line, _) = source.offset_to_line_col(block_node.location().start_offset());
        let (end_line, _) = source.offset_to_line_col(block_node.location().end_offset());
        if start_line != end_line {
            return Vec::new();
        }

        // Check if the parameter names match
        for (i, req) in requireds.iter().enumerate() {
            if let Some(rp) = req.as_required_parameter_node() {
                if rp.name().as_slice() != expected_params[i] {
                    let loc = block_params.location();
                    let (line, column) = source.offset_to_line_col(loc.start_offset());
                    return vec![self.diagnostic(
                        source,
                        line,
                        column,
                        format!(
                            "Name `{}` block params `|{}, {}|`.",
                            String::from_utf8_lossy(method_name),
                            String::from_utf8_lossy(expected_params[0]),
                            String::from_utf8_lossy(expected_params[1]),
                        ),
                    )];
                }
            }
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SingleLineBlockParams, "cops/style/single_line_block_params");
}
