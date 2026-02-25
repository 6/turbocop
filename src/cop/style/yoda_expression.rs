use crate::cop::node_type::CALL_NODE;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct YodaExpression;

impl Cop for YodaExpression {
    fn name(&self) -> &'static str {
        "Style/YodaExpression"
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

        // Check if LHS is a constant portion and RHS is not
        let lhs_constant = is_constant_portion(&receiver);
        let rhs_constant = is_constant_portion(&arg_list[0]);

        if lhs_constant && !rhs_constant {
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

fn is_constant_portion(node: &ruby_prism::Node<'_>) -> bool {
    // Match RuboCop's constant_portion? which checks :numeric and :const
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_constant_read_node().is_some()
        || node.as_constant_path_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(YodaExpression, "cops/style/yoda_expression");
}
