use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct MarshalLoad;

impl Cop for MarshalLoad {
    fn name(&self) -> &'static str {
        "Security/MarshalLoad"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
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

        let method = call.name().as_slice();
        if method != b"load" && method != b"restore" {
            return Vec::new();
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let is_marshal = is_constant_named(&recv, b"Marshal");
        if !is_marshal {
            return Vec::new();
        }

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Avoid using `Marshal.load`.".to_string(),
        )]
    }
}

fn is_constant_named(node: &ruby_prism::Node<'_>, name: &[u8]) -> bool {
    if let Some(cr) = node.as_constant_read_node() {
        return cr.name().as_slice() == name;
    }
    if let Some(cp) = node.as_constant_path_node() {
        if let Some(child) = cp.name() {
            if child.as_slice() == name && cp.parent().is_none() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(MarshalLoad, "cops/security/marshal_load");
}
