use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE};

pub struct EmptyLiteral;

impl Cop for EmptyLiteral {
    fn name(&self) -> &'static str {
        "Style/EmptyLiteral"
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
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let method_name = call_node.name();
        let method_bytes = method_name.as_slice();

        // Must be `new` or `[]`
        if method_bytes != b"new" && method_bytes != b"[]" {
            return;
        }

        // Must have a constant receiver: Array, Hash, or String
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return,
        };

        let const_name: Vec<u8> = if let Some(cr) = receiver.as_constant_read_node() {
            cr.name().as_slice().to_vec()
        } else if let Some(cp) = receiver.as_constant_path_node() {
            // Handle ::Array, ::Hash, ::String
            let child_name = match cp.name() {
                Some(n) => n.as_slice().to_vec(),
                None => return,
            };
            // Only allow if the parent is nil/cbase (top-level)
            if cp.parent().is_some() {
                return;
            }
            child_name
        } else {
            return;
        };

        // Must have no arguments (empty constructor)
        if let Some(args) = call_node.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if !arg_list.is_empty() {
                // Exception: Array.new with empty array arg or Array[] with empty
                return;
            }
        }

        // Must not have a block (Hash.new { |h, k| h[k] = [] })
        if call_node.block().is_some() {
            return;
        }

        let msg = match const_name.as_slice() {
            b"Array" if method_bytes == b"new" || method_bytes == b"[]" => {
                let src = String::from_utf8_lossy(call_node.location().as_slice());
                format!("Use array literal `[]` instead of `{}`.", src)
            }
            b"Hash" if method_bytes == b"new" || method_bytes == b"[]" => {
                let src = String::from_utf8_lossy(call_node.location().as_slice());
                format!("Use hash literal `{{}}` instead of `{}`.", src)
            }
            b"String" if method_bytes == b"new" => {
                "Use string literal `''` instead of `String.new`.".to_string()
            }
            _ => return,
        };

        let loc = call_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, msg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLiteral, "cops/style/empty_literal");
}
