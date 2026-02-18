use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SendWithLiteralMethodName;

impl Cop for SendWithLiteralMethodName {
    fn name(&self) -> &'static str {
        "Style/SendWithLiteralMethodName"
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

        // Check for public_send, __send__, or send (if not allowed)
        let is_target = name == b"public_send"
            || name == b"__send__"
            || (!allow_send && name == b"send");

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

        // First argument must be a static symbol or string
        let is_literal = arg_list[0].as_symbol_node().is_some()
            || arg_list[0].as_string_node().is_some();

        if !is_literal {
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
