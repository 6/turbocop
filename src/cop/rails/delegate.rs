use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct Delegate;

impl Cop for Delegate {
    fn name(&self) -> &'static str {
        "Rails/Delegate"
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
        let def_node = match node.as_def_node() {
            Some(d) => d,
            None => return Vec::new(),
        };

        // Must have no parameters (or empty parens)
        if let Some(params) = def_node.parameters() {
            let has_params = params.requireds().iter().next().is_some()
                || params.optionals().iter().next().is_some()
                || params.rest().is_some()
                || params.keywords().iter().next().is_some()
                || params.keyword_rest().is_some()
                || params.block().is_some();
            if has_params {
                return Vec::new();
            }
        }

        // Body must be a single call expression
        let body = match def_node.body() {
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

        let call = match body_nodes[0].as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have a receiver (delegating to another object)
        let receiver = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // The receiver should be a simple reference, not a chained call.
        // e.g., `client.name` is OK (client is receiverless), but
        // `client.name.upcase` is not (upcase's receiver is client.name).
        if let Some(recv_call) = receiver.as_call_node() {
            if recv_call.receiver().is_some() {
                return Vec::new();
            }
        }

        // The delegated call should have no arguments (simple forwarding)
        if call.arguments().is_some() {
            return Vec::new();
        }

        // Should not have a block
        if call.block().is_some() {
            return Vec::new();
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use `delegate` to define delegations.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Delegate, "cops/rails/delegate");
}
