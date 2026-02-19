use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE, STRING_NODE};

pub struct Open;

/// Check if a constant node matches a given name (handles both ConstantReadNode and ConstantPathNode).
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

/// Check if the argument is a "safe" string literal.
/// A safe argument is a non-empty string that doesn't start with '|'.
fn is_safe_arg(node: &ruby_prism::Node<'_>) -> bool {
    // Simple string literal
    if let Some(s) = node.as_string_node() {
        let content = s.unescaped();
        return !content.is_empty() && !content.starts_with(b"|");
    }
    // Interpolated string: check if first part is a safe string literal
    if let Some(dstr) = node.as_interpolated_string_node() {
        let parts: Vec<_> = dstr.parts().iter().collect();
        if let Some(first) = parts.first() {
            return is_safe_arg(first);
        }
    }
    // Concatenated string via + operator: check the receiver (left-hand side)
    if let Some(call) = node.as_call_node() {
        if call.name().as_slice() == b"+" {
            if let Some(recv) = call.receiver() {
                if recv.as_string_node().is_some() {
                    return is_safe_arg(&recv);
                }
            }
        }
    }
    false
}

impl Cop for Open {
    fn name(&self) -> &'static str {
        "Security/Open"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, INTERPOLATED_STRING_NODE, STRING_NODE]
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

        if call.name().as_slice() != b"open" {
            return;
        }

        // Determine if receiver matches: no receiver (bare open), Kernel, or URI
        let is_uri;
        match call.receiver() {
            None => {
                is_uri = false;
            }
            Some(recv) => {
                if is_constant_named(&recv, b"Kernel") {
                    is_uri = false;
                } else if is_constant_named(&recv, b"URI") {
                    is_uri = true;
                } else {
                    // Not a relevant receiver (e.g., File.open, obj.open)
                    return;
                }
            }
        };

        // Must have arguments; open() with no args is not a security risk
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return;
        }

        // Check if the first argument is a safe literal string
        if is_safe_arg(&arg_list[0]) {
            return;
        }

        let msg = if is_uri {
            "The use of `URI.open` is a serious security risk."
        } else {
            "The use of `Kernel#open` is a serious security risk."
        };

        let msg_loc = call.message_loc().unwrap();
        let (line, column) = source.offset_to_line_col(msg_loc.start_offset());
        diagnostics.push(self.diagnostic(source, line, column, msg.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(Open, "cops/security/open");
}
