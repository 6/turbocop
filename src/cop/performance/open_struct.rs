// RuboCop only flags OpenStruct.new(...) calls, not bare OpenStruct references.
// Pattern: (send (const {nil? cbase} :OpenStruct) :new ...)
use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct OpenStruct;

impl Cop for OpenStruct {
    fn name(&self) -> &'static str {
        "Performance/OpenStruct"
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
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        if call.name().as_slice() != b"new" {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Match bare `OpenStruct` or `::OpenStruct` (rooted constant path with no parent)
        let is_open_struct = if let Some(cr) = receiver.as_constant_read_node() {
            cr.name().as_slice() == b"OpenStruct"
        } else if let Some(cp) = receiver.as_constant_path_node() {
            cp.parent().is_none() && cp.name().map(|n| n.as_slice()) == Some(b"OpenStruct")
        } else {
            false
        };

        if !is_open_struct {
            return;
        }

        let loc = node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Use `Struct` instead of `OpenStruct`.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OpenStruct, "cops/performance/open_struct");
}
