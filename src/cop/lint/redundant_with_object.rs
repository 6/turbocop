use crate::cop::shared::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=2, FN=0.
///
/// FP=2: both mismatches used zero-parameter blocks (`{ 42 }`, `{ break 42 }`).
/// RuboCop only flags one-argument blocks plus `_1`/`it` implicit parameter forms;
/// zero-argument blocks can change `each_with_object` return semantics and must be
/// left alone. The earlier arguments-present guard for missing `with_object` args
/// remains covered by fixtures.
/// FN=0: no missing detections were reported for this cop in the corpus run.
pub struct RedundantWithObject;

impl Cop for RedundantWithObject {
    fn name(&self) -> &'static str {
        "Lint/RedundantWithObject"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE]
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call.name().as_slice();

        if method_name != b"each_with_object" {
            return;
        }

        // RuboCop only flags when the object argument is actually provided,
        // e.g. `each_with_object([])`.  Without arguments it's not redundant.
        let has_args = call
            .arguments()
            .is_some_and(|args| !args.arguments().is_empty());
        if !has_args {
            return;
        }

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        if redundant_block_signature(&block_node) {
            let msg_loc = call.message_loc().unwrap_or(call.location());
            let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
            let mut diag =
                self.diagnostic(source, line, column, "Redundant `with_object`.".to_string());
            // Autocorrect: replace `each_with_object(arg)` with `each`
            if let Some(ref mut corr) = corrections {
                // Replace method name and remove arguments
                let args_end = call.arguments().unwrap().location().end_offset();
                // Find closing paren after args
                let src = source.as_bytes();
                let mut end = args_end;
                if end < src.len() && src[end] == b')' {
                    end += 1;
                }
                corr.push(crate::correction::Correction {
                    start: msg_loc.start_offset(),
                    end,
                    replacement: "each".to_string(),
                    cop_name: self.name(),
                    cop_index: 0,
                });
                diag.corrected = true;
            }
            diagnostics.push(diag);
        }
    }
}

fn redundant_block_signature(block: &ruby_prism::BlockNode<'_>) -> bool {
    let Some(params) = block.parameters() else {
        return false;
    };

    if let Some(block_params) = params.as_block_parameters_node() {
        let Some(params_node) = block_params.parameters() else {
            return false;
        };

        return params_node.requireds().len() == 1
            && params_node.optionals().is_empty()
            && params_node.rest().is_none()
            && params_node.posts().is_empty()
            && params_node.keywords().is_empty()
            && params_node.keyword_rest().is_none()
            && params_node.block().is_none();
    }

    if let Some(numbered) = params.as_numbered_parameters_node() {
        return numbered.maximum() == 1;
    }

    params.as_it_parameters_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantWithObject, "cops/lint/redundant_with_object");
    crate::cop_autocorrect_fixture_tests!(RedundantWithObject, "cops/lint/redundant_with_object");
}
