use crate::cop::shared::node_type::{
    ARRAY_NODE, BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE, IT_LOCAL_VARIABLE_READ_NODE,
    IT_PARAMETERS_NODE, LOCAL_VARIABLE_READ_NODE, NUMBERED_PARAMETERS_NODE,
    REQUIRED_PARAMETER_NODE, STATEMENTS_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Performance/ZipWithoutBlock
///
/// Detects `map { |x| [x] }` / `collect { |x| [x] }` patterns that can be
/// replaced with `.zip`.
///
/// ## FN investigation (2026-03-04)
/// Root cause: only handled explicit `BlockParametersNode` (`|x|` style).
/// Missed `NumberedParametersNode` (`_1`) and `ItParametersNode` (`it`).
/// Fix: added branches for both implicit parameter styles, checking that the
/// block body is `[_1]` / `[it]` respectively.
pub struct ZipWithoutBlock;

impl Cop for ZipWithoutBlock {
    fn name(&self) -> &'static str {
        "Performance/ZipWithoutBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            BLOCK_NODE,
            BLOCK_PARAMETERS_NODE,
            CALL_NODE,
            IT_LOCAL_VARIABLE_READ_NODE,
            IT_PARAMETERS_NODE,
            LOCAL_VARIABLE_READ_NODE,
            NUMBERED_PARAMETERS_NODE,
            REQUIRED_PARAMETER_NODE,
            STATEMENTS_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        // Look for CallNode .map or .collect with a block
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.receiver().is_none() {
            return;
        }

        let method_name = call.name().as_slice();
        if method_name != b"map" && method_name != b"collect" {
            return;
        }

        // Must have a block (not a block argument like &method)
        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(bn) => bn,
            None => return,
        };

        // Check block parameter style and verify body is `[param]`
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        // Get the single body statement (must be a 1-element array)
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return,
        };

        let body_stmts = stmts.body();
        if body_stmts.len() != 1 {
            return;
        }

        let stmt = match body_stmts.iter().next() {
            Some(s) => s,
            None => return,
        };

        let array = match stmt.as_array_node() {
            Some(a) => a,
            None => return,
        };

        let elements = array.elements();
        if elements.len() != 1 {
            return;
        }

        let elem = match elements.iter().next() {
            Some(e) => e,
            None => return,
        };

        if params.as_numbered_parameters_node().is_some() {
            // Numbered parameters: body must be [_1]
            let local_var = match elem.as_local_variable_read_node() {
                Some(lv) => lv,
                None => return,
            };
            if local_var.name().as_slice() != b"_1" {
                return;
            }
        } else if params.as_it_parameters_node().is_some() {
            // Ruby 3.4+ `it` implicit parameter: body must be [it]
            if elem.as_it_local_variable_read_node().is_none() {
                return;
            }
        } else if let Some(block_params) = params.as_block_parameters_node() {
            // Explicit block parameters: body must be [param]
            let param_list = match block_params.parameters() {
                Some(pl) => pl,
                None => return,
            };

            let requireds = param_list.requireds();
            if requireds.len() != 1 {
                return;
            }

            let first_param = match requireds.iter().next() {
                Some(p) => p,
                None => return,
            };

            let param_name = match first_param.as_required_parameter_node() {
                Some(rp) => rp.name(),
                None => return,
            };

            let local_var = match elem.as_local_variable_read_node() {
                Some(lv) => lv,
                None => return,
            };

            if local_var.name().as_slice() != param_name.as_slice() {
                return;
            }
        } else {
            return;
        }

        // Offense spans from the method name selector to the end of the block
        let msg_loc = match call.message_loc() {
            Some(loc) => loc,
            None => return,
        };

        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `zip` without a block argument instead.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ZipWithoutBlock, "cops/performance/zip_without_block");
}
