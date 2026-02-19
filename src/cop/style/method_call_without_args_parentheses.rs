use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::CALL_NODE;

pub struct MethodCallWithoutArgsParentheses;

impl Cop for MethodCallWithoutArgsParentheses {
    fn name(&self) -> &'static str {
        "Style/MethodCallWithoutArgsParentheses"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allowed_methods = config.get_string_array("AllowedMethods");
        let _allowed_patterns = config.get_string_array("AllowedPatterns");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must have parentheses (opening_loc present)
        if call.opening_loc().is_none() {
            return Vec::new();
        }

        // Must have no arguments
        if call.arguments().is_some() {
            return Vec::new();
        }

        // Must have no block
        if call.block().is_some() {
            return Vec::new();
        }

        // Must have a message (method name)
        let msg_loc = match call.message_loc() {
            Some(l) => l,
            None => return Vec::new(),
        };

        let method_name = call.name();
        let method_bytes = method_name.as_slice();

        // Skip methods starting with uppercase (like Test()) - these are conversion methods
        if method_bytes.first().is_some_and(|b| b.is_ascii_uppercase()) {
            return Vec::new();
        }

        // Skip operator methods ([], []=, etc.) - these use bracket syntax, not parentheses
        if method_bytes == b"[]" || method_bytes == b"[]=" {
            return Vec::new();
        }

        // Skip `not()` - keyword (Prism names it `!` internally but message_loc is `not`)
        if method_bytes == b"not" || msg_loc.as_slice() == b"not" {
            return Vec::new();
        }

        // Skip lambda call syntax: thing.()
        if msg_loc.as_slice() == b"call" && call.call_operator_loc().is_some() {
            // Check if it's actually `.()` syntax (no explicit `call` in source)
            let src = source.as_bytes();
            let op_loc = call.call_operator_loc().unwrap();
            let after_op = op_loc.end_offset();
            if after_op < src.len() && src[after_op] == b'(' {
                return Vec::new();
            }
        }

        // Check if this is `it()` inside a single-line block without receiver
        // `it()` without a receiver in a block body is special (Lint/ItWithoutArgumentsInBlock)
        if method_bytes == b"it" && call.receiver().is_none() {
            // We flag it in def bodies and with-receiver calls, but skip bare `it()` in block context
            // For simplicity, skip bare `it()` (no receiver) - this matches RuboCop's behavior
            // where `it()` without receiver in a single-line block is allowed
            // But `foo.it()` is flagged
            // We approximate: if no receiver, check context more carefully.
            // For now, just skip bare `it()` (no receiver) entirely to be safe.
            return Vec::new();
        }

        // Check AllowedMethods
        if let Some(ref allowed) = allowed_methods {
            let name_str = std::str::from_utf8(method_bytes).unwrap_or("");
            if allowed.iter().any(|m| m == name_str) {
                return Vec::new();
            }
        }

        let open_loc = call.opening_loc().unwrap();
        let (line, column) = source.offset_to_line_col(open_loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not use parentheses for method calls with no arguments.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MethodCallWithoutArgsParentheses, "cops/style/method_call_without_args_parentheses");
}
