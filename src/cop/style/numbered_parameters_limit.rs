use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use ruby_prism::Visit;
use std::collections::HashSet;
use crate::cop::node_type::{BLOCK_NODE, CALL_NODE, NUMBERED_PARAMETERS_NODE};

pub struct NumberedParametersLimit;

/// Count unique numbered parameter references (_1.._9) in a block body.
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

    // Don't descend into nested blocks (they have their own numbered params scope)
    fn visit_block_node(&mut self, _node: &ruby_prism::BlockNode<'pr>) {
        // Stop recursion into nested blocks
    }

    fn visit_lambda_node(&mut self, _node: &ruby_prism::LambdaNode<'pr>) {
        // Stop recursion into nested lambdas
    }
}

impl Cop for NumberedParametersLimit {
    fn name(&self) -> &'static str {
        "Style/NumberedParametersLimit"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[BLOCK_NODE, CALL_NODE, NUMBERED_PARAMETERS_NODE]
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
        let max = config.get_usize("Max", 1);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let block = match call.block() {
            Some(b) => b,
            None => return,
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return,
        };

        // In Prism, blocks with numbered params have parameters() set to a
        // NumberedParametersNode. Check for it to confirm this is a numbered params block.
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return,
        };

        if params.as_numbered_parameters_node().is_none() {
            return;
        }

        // Count unique numbered parameter references in the block body.
        // RuboCop counts unique _N references, not the highest N.
        // So `{ _2 }` has 1 unique param (OK with max=1),
        // but `{ _1 + _2 }` has 2 unique params (offense with max=1).
        let body = match block_node.body() {
            Some(b) => b,
            None => return,
        };

        let unique_count = count_unique_numbered_params(&body);

        if unique_count > max {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                format!("Avoid using more than {max} numbered parameters; {unique_count} detected."),
            ));
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(NumberedParametersLimit, "cops/style/numbered_parameters_limit");
}
