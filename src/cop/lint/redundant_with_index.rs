use crate::cop::shared::node_type::{BLOCK_NODE, BLOCK_PARAMETERS_NODE, CALL_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// ## Corpus investigation (2026-03-08)
///
/// Corpus oracle reported FP=13, FN=1.
///
/// FP=13: RuboCop only flags one-parameter blocks plus `_1`/`it` implicit
/// parameter forms. Zero-parameter blocks, splat-only blocks, and `_1; _2`
/// numbered-parameter blocks all changed behavior and were falsely flagged by
/// our old `param_count < 2` heuristic.
///
/// ## Corpus investigation (2026-03-25)
///
/// FP=1: Blocks with destructured parameters like `|(a, c)|` use a
/// `MultiTargetNode` in Prism's requireds list (count still 1), but RuboCop's
/// `(args (arg _))` pattern only matches simple arg nodes. Fixed by checking
/// that the single required param is a `RequiredParameterNode`.
///
/// FN=1: the remaining miss was the chained `with_index` form
/// (`receiver.each.with_index { |item| ... }` / `times.with_index { |i| ... }`),
/// which RuboCop treats the same as `each_with_index` as long as `with_index`
/// is called on another call node rather than directly on a receiver.
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
        if method_name != b"each_with_index" && method_name != b"with_index" {
            return;
        }

        if method_name == b"with_index" {
            let Some(receiver) = call.receiver() else {
                return;
            };
            let Some(receiver_call) = receiver.as_call_node() else {
                return;
            };
            if receiver_call.receiver().is_none() {
                return;
            }
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
                self.diagnostic(source, line, column, "Redundant `with_index`.".to_string());
            // Autocorrect: replace `each_with_index` with `each`, or remove `.with_index`
            if let Some(ref mut corr) = corrections {
                if method_name == b"each_with_index" {
                    corr.push(crate::correction::Correction {
                        start: msg_loc.start_offset(),
                        end: msg_loc.end_offset(),
                        replacement: "each".to_string(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                } else {
                    // `with_index` — remove `.with_index` from chained call
                    // The receiver ends before `.with_index`
                    let receiver = call.receiver().unwrap();
                    corr.push(crate::correction::Correction {
                        start: receiver.location().end_offset(),
                        end: msg_loc.end_offset(),
                        replacement: String::new(),
                        cop_name: self.name(),
                        cop_index: 0,
                    });
                }
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

        if params_node.requireds().len() != 1 {
            return false;
        }
        // Destructured params like |(a, c)| are MultiTargetNode, not RequiredParameterNode.
        // RuboCop only matches simple (arg _), so skip destructured params.
        let req = params_node.requireds().iter().next().unwrap();
        if req.as_required_parameter_node().is_none() {
            return false;
        }

        return params_node.optionals().is_empty()
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
    crate::cop_fixture_tests!(RedundantWithIndex, "cops/lint/redundant_with_index");
    crate::cop_autocorrect_fixture_tests!(RedundantWithIndex, "cops/lint/redundant_with_index");
}
