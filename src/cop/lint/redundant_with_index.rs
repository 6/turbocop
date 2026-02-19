use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE};

pub struct RedundantWithIndex;

impl Cop for RedundantWithIndex {
    fn name(&self) -> &'static str {
        "Lint/RedundantWithIndex"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call.name().as_slice();

        // Check each_with_index { |x| } (only 1 block param, index unused)
        if method_name != b"each_with_index" {
            return Vec::new();
        }

        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let params = block_node.parameters();
        let param_count = match &params {
            Some(p) => {
                if let Some(bp) = p.as_block_parameters_node() {
                    if let Some(params_node) = bp.parameters() {
                        params_node.requireds().len()
                            + params_node.optionals().len()
                    } else {
                        0
                    }
                } else {
                    0
                }
            }
            None => 0,
        };

        if param_count < 2 {
            let msg_loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            return vec![self.diagnostic(
                source,
                line,
                column,
                "Redundant `with_index`.".to_string(),
            )];
        }

        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantWithIndex, "cops/lint/redundant_with_index");
}
