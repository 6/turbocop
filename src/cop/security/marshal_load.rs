use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};

pub struct MarshalLoad;

impl Cop for MarshalLoad {
    fn name(&self) -> &'static str {
        "Security/MarshalLoad"
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
        if method != b"load" && method != b"restore" {
            return;
        }

        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let is_marshal = is_constant_named(&recv, b"Marshal");
        if !is_marshal {
            return;
        }

        // Exclude the "deep copy hack" pattern: Marshal.load(Marshal.dump(...))
        if let Some(args) = call.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if let Some(first_arg) = arg_list.first() {
                if let Some(inner_call) = first_arg.as_call_node() {
                    if inner_call.name().as_slice() == b"dump" {
                        if let Some(inner_recv) = inner_call.receiver() {
                            if is_constant_named(&inner_recv, b"Marshal") {
                                return;
                            }
                        }
                    }
                }
            }
        }

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Avoid using `Marshal.load`.".to_string(),
        ));
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
