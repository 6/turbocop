use ruby_prism::Visit;

use crate::cop::node_type::DEF_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct BlockGivenWithExplicitBlock;

impl Cop for BlockGivenWithExplicitBlock {
    fn name(&self) -> &'static str {
        "Performance/BlockGivenWithExplicitBlock"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[DEF_NODE]
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
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return,
        };

        // Check if method has an explicit &block parameter
        let params = match def_node.parameters() {
            Some(p) => p,
            None => return,
        };

        let block_param = match params.block() {
            Some(b) => b,
            None => return,
        };

        // Skip anonymous block forwarding (`&` without a name, Ruby 3.1+)
        let block_name = match block_param.name() {
            Some(n) => n,
            None => return,
        };

        // Walk the body looking for `block_given?` calls
        let body = match def_node.body() {
            Some(b) => b,
            None => return,
        };

        // Check if the block param is reassigned in the body — if so, skip
        let mut reassign_finder = ReassignFinder {
            name: block_name.as_slice(),
            found: false,
        };
        reassign_finder.visit(&body);
        if reassign_finder.found {
            return;
        }

        let mut finder = BlockGivenFinder {
            offsets: Vec::new(),
        };
        finder.visit(&body);

        for offset in finder.offsets {
            let (line, column) = source.offset_to_line_col(offset);
            diagnostics.push(self.diagnostic(source, line, column, "Check `block` instead of using `block_given?` with explicit `&block` parameter.".to_string()));
        }
    }
}

struct BlockGivenFinder {
    offsets: Vec<usize>,
}

impl<'pr> Visit<'pr> for BlockGivenFinder {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        if node.name().as_slice() == b"block_given?"
            && node.receiver().is_none()
            && node.arguments().is_none()
        {
            self.offsets.push(node.location().start_offset());
        }
        // Recurse into children to find block_given? inside negation,
        // method arguments, ternary conditions, etc.
        ruby_prism::visit_call_node(self, node);
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Don't recurse into nested method definitions
    }
}

/// Detects reassignment of the block parameter variable within the method body.
/// When the block param is reassigned (e.g., `block ||= -> {}`, `block = proc {}`),
/// RuboCop suppresses the offense because `block_given?` may behave differently
/// from checking the reassigned variable.
struct ReassignFinder<'a> {
    name: &'a [u8],
    found: bool,
}

impl<'pr> Visit<'pr> for ReassignFinder<'_> {
    fn visit_local_variable_write_node(&mut self, node: &ruby_prism::LocalVariableWriteNode<'pr>) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_write_node(self, node);
    }

    fn visit_local_variable_or_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOrWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_or_write_node(self, node);
    }

    fn visit_local_variable_operator_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableOperatorWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_operator_write_node(self, node);
    }

    fn visit_local_variable_and_write_node(
        &mut self,
        node: &ruby_prism::LocalVariableAndWriteNode<'pr>,
    ) {
        if node.name().as_slice() == self.name {
            self.found = true;
        }
        ruby_prism::visit_local_variable_and_write_node(self, node);
    }

    fn visit_def_node(&mut self, _node: &ruby_prism::DefNode<'pr>) {
        // Don't recurse into nested method definitions (different scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        BlockGivenWithExplicitBlock,
        "cops/performance/block_given_with_explicit_block"
    );
}
