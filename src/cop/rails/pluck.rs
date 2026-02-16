use ruby_prism::Visit;

use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Pluck;

impl Cop for Pluck {
    fn name(&self) -> &'static str {
        "Rails/Pluck"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn check_source(
        &self,
        source: &SourceFile,
        parse_result: &ruby_prism::ParseResult<'_>,
        _code_map: &crate::parse::codemap::CodeMap,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let mut visitor = PluckVisitor {
            cop: self,
            source,
            // Track depth of blocks that have a receiver (i.e., `receiver.method { ... }`)
            receiver_block_depth: 0,
            diagnostics: Vec::new(),
        };
        visitor.visit(&parse_result.node());
        visitor.diagnostics
    }
}

struct PluckVisitor<'a, 'src> {
    cop: &'a Pluck,
    source: &'src SourceFile,
    receiver_block_depth: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'pr> Visit<'pr> for PluckVisitor<'_, '_> {
    fn visit_call_node(&mut self, node: &ruby_prism::CallNode<'pr>) {
        let method_name = node.name().as_slice();

        // Check for pluck candidate pattern: receiver.map/collect { |x| x[:key] }
        if (method_name == b"map" || method_name == b"collect") && self.receiver_block_depth == 0 {
            if let Some(diag) = self.check_pluck_candidate(node) {
                self.diagnostics.push(diag);
            }
        }

        // Track receiver block depth for nested blocks
        if let Some(block) = node.block() {
            if let Some(block_node) = block.as_block_node() {
                // If this call has a receiver, nested map/collect should be skipped
                // to prevent N+1 query issues (matching RuboCop behavior).
                if node.receiver().is_some() {
                    self.receiver_block_depth += 1;
                    ruby_prism::visit_block_node(self, &block_node);
                    self.receiver_block_depth -= 1;
                } else {
                    ruby_prism::visit_block_node(self, &block_node);
                }
                // Visit remaining children (receiver, arguments) but not the block again
                if let Some(recv) = node.receiver() {
                    self.visit(&recv);
                }
                if let Some(args) = node.arguments() {
                    self.visit_arguments_node(&args);
                }
                return;
            }
        }

        // Default: visit all children
        ruby_prism::visit_call_node(self, node);
    }
}

impl PluckVisitor<'_, '_> {
    fn check_pluck_candidate(&self, call: &ruby_prism::CallNode<'_>) -> Option<Diagnostic> {
        // Must have a block
        let block = call.block()?;
        let block_node = block.as_block_node()?;

        // Get block parameter name (must have exactly one)
        let params = block_node.parameters()?;
        let block_params = params.as_block_parameters_node()?;
        let param_list = block_params.parameters()?;
        let requireds: Vec<_> = param_list.requireds().iter().collect();
        if requireds.len() != 1 {
            return None;
        }
        let param_node = requireds[0].as_required_parameter_node()?;
        let param_name = param_node.name().as_slice();

        // Block body should be a single indexing operation: block_param[:key]
        let body = block_node.body()?;
        let stmts = body.as_statements_node()?;
        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return None;
        }

        let inner_call = body_nodes[0].as_call_node()?;
        if inner_call.name().as_slice() != b"[]" {
            return None;
        }

        // Receiver of [] must be the block parameter (a local variable read)
        let receiver = inner_call.receiver()?;
        let lvar = receiver.as_local_variable_read_node()?;
        if lvar.name().as_slice() != param_name {
            return None;
        }

        let loc = call.location();
        let (line, column) = self.source.offset_to_line_col(loc.start_offset());
        Some(self.cop.diagnostic(
            self.source,
            line,
            column,
            "Use `pluck(:key)` instead of `map { |item| item[:key] }`.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Pluck, "cops/rails/pluck");
}
