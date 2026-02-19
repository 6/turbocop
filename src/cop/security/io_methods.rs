use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};

pub struct IoMethods;

const DANGEROUS_METHODS: &[&[u8]] = &[
    b"read", b"write", b"binread", b"binwrite", b"foreach", b"readlines",
];

impl Cop for IoMethods {
    fn name(&self) -> &'static str {
        "Security/IoMethods"
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
    ) {
        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method = call.name().as_slice();
        if !DANGEROUS_METHODS.iter().any(|&m| m == method) {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_io = if let Some(cr) = recv.as_constant_read_node() {
            cr.name().as_slice() == b"IO"
        } else if let Some(cp) = recv.as_constant_path_node() {
            cp.name().map(|n| n.as_slice() == b"IO").unwrap_or(false)
                && cp.parent().is_none()
        } else {
            false
        };

        if !is_io {
            return;
        }

        let method_str = std::str::from_utf8(method).unwrap_or("");
        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("The use of `IO.{method_str}` is a security risk."),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(IoMethods, "cops/security/io_methods");
}
