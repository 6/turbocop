use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, STRING_NODE, SYMBOL_NODE};

pub struct SendWithLiteralMethodName;

impl Cop for SendWithLiteralMethodName {
    fn name(&self) -> &'static str {
        "Style/SendWithLiteralMethodName"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, STRING_NODE, SYMBOL_NODE]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let allow_send = config.get_bool("AllowSend", true);

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let name = call.name().as_slice();

        // Check for public_send, __send__, or send
        // When AllowSend is true (default), only public_send is flagged.
        // When AllowSend is false, send and __send__ are also flagged.
        let is_target = name == b"public_send"
            || (!allow_send && (name == b"__send__" || name == b"send"));

        if !is_target {
            return Vec::new();
        }

        let args = match call.arguments() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.is_empty() {
            return Vec::new();
        }

        // First argument must be a static symbol or string with a valid method name.
        // Strings with spaces or special chars are NOT valid method names.
        let is_valid_literal = if let Some(sym) = arg_list[0].as_symbol_node() {
            let name = sym.unescaped();
            // Symbol must be a valid method name (no spaces)
            !name.contains(&b' ')
        } else if let Some(s) = arg_list[0].as_string_node() {
            let content = s.unescaped();
            // String must be a valid method name (no spaces, not empty)
            !content.is_empty() && !content.contains(&b' ')
        } else {
            false
        };

        if !is_valid_literal {
            return Vec::new();
        }

        let loc = call.message_loc().unwrap_or(call.location());
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Use a direct method call instead of `send` with a literal method name.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SendWithLiteralMethodName, "cops/style/send_with_literal_method_name");
}
