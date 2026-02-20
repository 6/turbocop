use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE, SYMBOL_NODE};

pub struct SendWithMixinArgument;

const SEND_METHODS: &[&[u8]] = &[b"send", b"public_send", b"__send__"];
const MIXIN_METHODS: &[&[u8]] = &[b"include", b"prepend", b"extend"];

impl Cop for SendWithMixinArgument {
    fn name(&self) -> &'static str {
        "Lint/SendWithMixinArgument"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, CONSTANT_PATH_NODE, CONSTANT_READ_NODE, STRING_NODE, SYMBOL_NODE]
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

        let method_name = call.name().as_slice();

        // Must be send/public_send/__send__
        if !SEND_METHODS.iter().any(|m| *m == method_name) {
            return;
        }

        // Must have a receiver (constant)
        let recv = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        // Receiver should be a constant
        if recv.as_constant_read_node().is_none() && recv.as_constant_path_node().is_none() {
            return;
        }

        // Must have at least 2 arguments: the mixin method name and the module
        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() < 2 {
            return;
        }

        // First argument must be a symbol or string that's a mixin method
        let first_arg = &arg_list[0];
        let mixin_name = if let Some(sym) = first_arg.as_symbol_node() {
            sym.unescaped().to_vec()
        } else if let Some(s) = first_arg.as_string_node() {
            s.unescaped().to_vec()
        } else {
            return;
        };

        if !MIXIN_METHODS.iter().any(|m| **m == *mixin_name) {
            return;
        }

        // Second argument should be a constant
        let second_arg = &arg_list[1];
        if second_arg.as_constant_read_node().is_none()
            && second_arg.as_constant_path_node().is_none()
        {
            return;
        }

        let mixin_str = std::str::from_utf8(&mixin_name).unwrap_or("include");
        let module_name =
            std::str::from_utf8(second_arg.location().as_slice()).unwrap_or("Module");

        // Build the "bad method" string for the message
        // Only include the method name and arguments, not the receiver
        let method_str = std::str::from_utf8(method_name).unwrap_or("send");
        let args_src = if let Some(args_loc) = call.arguments().map(|a| a.location()) {
            std::str::from_utf8(args_loc.as_slice()).unwrap_or("...")
        } else {
            "..."
        };

        let loc = call.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!(
                "Use `{mixin_str} {module_name}` instead of `{method_str}({args_src})`."
            ),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(SendWithMixinArgument, "cops/lint/send_with_mixin_argument");
}
