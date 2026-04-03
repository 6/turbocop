use crate::cop::shared::node_type::{BLOCK_NODE, CALL_NODE, LAMBDA_NODE, NUMBERED_PARAMETERS_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;

/// Fixes two Prism mismatches versus RuboCop:
/// `-> { _1 + _2 }` is a `LambdaNode`, and chained numblocks like
/// `foo.map { _1 }.select { _2 }` must count numbered params across the full
/// numblock subtree, including numbered params in the receiver chain.
pub struct NumberedParametersLimit;

/// Count unique numbered parameter references (_1.._9) in the subtree RuboCop
/// inspects for a numbered block or lambda.
fn count_unique_numbered_params(node: &ruby_prism::Node<'_>) -> usize {
    let mut finder = NumberedParamFinder {
        found: HashSet::new(),
    };
    finder.visit(node);
    finder.found.len()
}

struct NumberedParamFinder {
    found: HashSet<u8>,
}

impl<'pr> Visit<'pr> for NumberedParamFinder {
    fn visit_local_variable_read_node(&mut self, node: &ruby_prism::LocalVariableReadNode<'pr>) {
        let name = node.name().as_slice();
        // Match _1 through _9
        if name.len() == 2 && name[0] == b'_' && name[1] >= b'1' && name[1] <= b'9' {
            self.found.insert(name[1]);
        }
    }
}

fn has_numbered_parameters(parameters: Option<ruby_prism::Node<'_>>) -> bool {
    parameters
        .and_then(|params| params.as_numbered_parameters_node())
        .is_some()
}

fn diagnostic_message(max: usize, unique_count: usize) -> String {
    let parameter = if max == 1 { "parameter" } else { "parameters" };
    format!("Avoid using more than {max} numbered {parameter}; {unique_count} detected.")
}

impl Cop for NumberedParametersLimit {
    fn name(&self) -> &'static str {
        "Style/NumberedParametersLimit"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, LAMBDA_NODE, NUMBERED_PARAMETERS_NODE]
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
        let max = config.get_usize("Max", 1).min(9);

        let (location, unique_count) = if let Some(lambda) = node.as_lambda_node() {
            if !has_numbered_parameters(lambda.parameters()) {
                return;
            }

            (lambda.location(), count_unique_numbered_params(node))
        } else {
            let call = match node.as_call_node() {
                Some(c) => c,
                None => return,
            };

            let block_node = match call.block().and_then(|block| block.as_block_node()) {
                Some(block) => block,
                None => return,
            };

            // In Prism, blocks with numbered params have parameters() set to a
            // NumberedParametersNode. Check for it to confirm this is a numbered params block.
            if !has_numbered_parameters(block_node.parameters()) {
                return;
            }

            (call.location(), count_unique_numbered_params(node))
        };

        if unique_count > max {
            let (line, column) = source.offset_to_line_col(location.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                diagnostic_message(max, unique_count),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        NumberedParametersLimit,
        "cops/style/numbered_parameters_limit"
    );
}
