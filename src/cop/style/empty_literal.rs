use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLiteral;

impl Cop for EmptyLiteral {
    fn name(&self) -> &'static str {
        "Style/EmptyLiteral"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method_name = call_node.name();
        let method_bytes = method_name.as_slice();

        // Must be `new` or `[]`
        if method_bytes != b"new" && method_bytes != b"[]" {
            return Vec::new();
        }

        // Must have a constant receiver: Array, Hash, or String
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let const_name: Vec<u8> = if let Some(cr) = receiver.as_constant_read_node() {
            cr.name().as_slice().to_vec()
        } else if let Some(cp) = receiver.as_constant_path_node() {
            // Handle ::Array, ::Hash, ::String
            let child_name = match cp.name() {
                Some(n) => n.as_slice().to_vec(),
                None => return Vec::new(),
            };
            // Only allow if the parent is nil/cbase (top-level)
            if cp.parent().is_some() {
                return Vec::new();
            }
            child_name
        } else {
            return Vec::new();
        };

        // Must have no arguments (empty constructor)
        if let Some(args) = call_node.arguments() {
            let arg_list: Vec<_> = args.arguments().iter().collect();
            if !arg_list.is_empty() {
                // Exception: Array.new with empty array arg or Array[] with empty
                return Vec::new();
            }
        }

        // Must not have a block (Hash.new { |h, k| h[k] = [] })
        if call_node.block().is_some() {
            return Vec::new();
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
            _ => return Vec::new(),
        };

        let loc = call_node.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(source, line, column, msg)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLiteral, "cops/style/empty_literal");
}
