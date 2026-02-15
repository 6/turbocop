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
        if method_name != b"map" && method_name != b"collect" {
            return Vec::new();
        }

        // Must have a block
        let block = match call.block() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let block_node = match block.as_block_node() {
            Some(b) => b,
            None => return Vec::new(),
        };

        // Get block parameter name (must have exactly one)
        let params = match block_node.parameters() {
            Some(p) => p,
            None => return Vec::new(),
        };
        let block_params = match params.as_block_parameters_node() {
            Some(bp) => bp,
            None => return Vec::new(),
        };
        let param_list = match block_params.parameters() {
            Some(pl) => pl,
            None => return Vec::new(),
        };
        let requireds: Vec<_> = param_list.requireds().iter().collect();
        if requireds.len() != 1 {
            return Vec::new();
        }
        let param_node = match requireds[0].as_required_parameter_node() {
            Some(p) => p,
            None => return Vec::new(),
        };
        let param_name = param_node.name().as_slice();

        // Block body should be a single indexing operation: block_param[:key]
        let body = match block_node.body() {
            Some(b) => b,
            None => return Vec::new(),
        };

        let stmts = match body.as_statements_node() {
            Some(s) => s,
            None => return Vec::new(),
        };

        let body_nodes: Vec<_> = stmts.body().iter().collect();
        if body_nodes.len() != 1 {
            return Vec::new();
        }

        let inner_call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        if inner_call.name().as_slice() != b"[]" {
            return Vec::new();
        }

        // Receiver of [] must be the block parameter (a local variable read)
        let receiver = match inner_call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let lvar = match receiver.as_local_variable_read_node() {
            Some(l) => l,
            None => return Vec::new(),
        };

        if lvar.name().as_slice() != param_name {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `pluck(:key)` instead of `map { |item| item[:key] }`.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Pluck, "cops/rails/pluck");
}
