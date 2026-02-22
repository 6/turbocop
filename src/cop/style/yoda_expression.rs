use crate::cop::node_type::{
    CALL_NODE, FALSE_NODE, FLOAT_NODE, IMAGINARY_NODE, INTEGER_NODE, NIL_NODE, RATIONAL_NODE,
    STRING_NODE, SYMBOL_NODE, TRUE_NODE,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct YodaExpression;

impl Cop for YodaExpression {
    fn name(&self) -> &'static str {
        "Style/YodaExpression"
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            CALL_NODE,
            FALSE_NODE,
            FLOAT_NODE,
            IMAGINARY_NODE,
            INTEGER_NODE,
            NIL_NODE,
            RATIONAL_NODE,
            STRING_NODE,
            SYMBOL_NODE,
            TRUE_NODE,
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let supported_operators = config.get_string_array("SupportedOperators");

        let call = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        let name = call.name().as_slice();
        let name_str = match std::str::from_utf8(name) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Check if operator is in supported list (default: *, +, &, |, ^)
        let is_supported = if let Some(ref ops) = supported_operators {
            ops.iter().any(|op| op == name_str)
        } else {
            matches!(name, b"*" | b"+" | b"&" | b"|" | b"^")
        };

        if !is_supported {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let args = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let arg_list: Vec<_> = args.arguments().iter().collect();
        if arg_list.len() != 1 {
            return;
        }

        // Check if LHS is a literal and RHS is not
        let lhs_literal = is_literal(&receiver);
        let rhs_literal = is_literal(&arg_list[0]);

        if lhs_literal && !rhs_literal {
            let loc = node.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            diagnostics.push(self.diagnostic(
                source,
                line,
                column,
                "Prefer placing the expression on the left side of the operator.".to_string(),
            ));
        }
    }
}

fn is_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(YodaExpression, "cops/style/yoda_expression");
}
