use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct JsonLoad;

impl Cop for JsonLoad {
    fn name(&self) -> &'static str {
        "Security/JSONLoad"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE]
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

        if call.name().as_slice() != b"load" {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_json = is_constant_named(source, &recv, b"JSON");
        if !is_json {
            return;
        }

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Prefer `JSON.parse` over `JSON.load`.".to_string(),
        ));
    }
}

fn is_constant_named(_source: &SourceFile, node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == name;
    }
    if let Some(cp) = node.as_constant_path_node() {
        // Check if the last segment is the target name
        if let Some(child) = cp.name() {
            if child.as_slice() == name {
                // For ::JSON, parent is None; for Foo::JSON, parent is Some
                return cp.parent().is_none();
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(JsonLoad, "cops/security/json_load");
}
