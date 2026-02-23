use crate::cop::node_type::{CALL_NODE, INTEGER_NODE, PARENTHESES_NODE};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EvenOdd;

impl Cop for EvenOdd {
    fn name(&self) -> &'static str {
        "Style/EvenOdd"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, INTEGER_NODE, PARENTHESES_NODE]
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

        // Must be == or !=
        if method_bytes != b"==" && method_bytes != b"!=" {
            return;
        }

        // Receiver must be `x % 2` or `(x % 2)`
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return,
        };

        // Unwrap optional parentheses
        let modulo_call = if let Some(parens) = receiver.as_parentheses_node() {
            match parens.body() {
                Some(body) => body.as_call_node(),
                None => return,
            }
        } else {
            receiver.as_call_node()
        };

        let modulo_call = match modulo_call {
            Some(c) => c,
            None => return,
        };

        // Must be % method
        if modulo_call.name().as_slice() != b"%" {
            return;
        }

        // Argument of % must be integer literal 2
        let mod_args = match modulo_call.arguments() {
            Some(a) => a,
            None => return,
        };
        let mod_arg_list: Vec<_> = mod_args.arguments().iter().collect();
        if mod_arg_list.len() != 1 {
            return;
        }
        let mod_arg = &mod_arg_list[0];
        let int_node = match mod_arg.as_integer_node() {
            Some(i) => i,
            None => return,
        };
        let int_src = int_node.location().as_slice();
        if int_src != b"2" {
            return;
        }

        // The comparison argument must be integer literal 0 or 1
        let args = match call_node.arguments() {
            Some(a) => a,
            None => return,
        };
        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }
        let cmp_arg = &arg_list[0];
        let cmp_int = match cmp_arg.as_integer_node() {
            Some(i) => i,
            None => return,
        };
        let cmp_src = cmp_int.location().as_slice();
        if cmp_src != b"0" && cmp_src != b"1" {
            return;
        }

        let is_zero = cmp_src == b"0";
        let is_eq = method_bytes == b"==";

        let replacement = if is_zero {
            if is_eq { "even" } else { "odd" }
        } else {
            // comparing to 1
            if is_eq { "odd" } else { "even" }
        };

        let (line, column) = source.offset_to_line_col(call_node.location().start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            format!("Replace with `Integer#{}?`.", replacement),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EvenOdd, "cops/style/even_odd");
}
