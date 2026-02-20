use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct BindCall;

impl Cop for BindCall {
    fn name(&self) -> &'static str {
        "Performance/BindCall"
    }

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
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
        // Detect: foo.method(:bar).bind(obj).call
        // 3-level chain: method -> bind -> call
        let outer_call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if outer_call.name().as_slice() != b"call" {
            return;
        }

        let mid_node = match outer_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let mid_call = match mid_node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if mid_call.name().as_slice() != b"bind" {
            return;
        }

        let inner_node = match mid_call.receiver() {
            Some(r) => r,
            None => return,
        };

        let inner_call = match inner_node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if inner_call.name().as_slice() != b"method" {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, "Use `bind_call` instead of `method.bind.call`.".to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BindCall, "cops/performance/bind_call");
}
