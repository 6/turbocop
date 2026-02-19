use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use crate::cop::node_type::{CALL_NODE, FLOAT_NODE, INTEGER_NODE, NIL_NODE};

pub struct FloatComparison;

impl Cop for FloatComparison {
    fn name(&self) -> &'static str {
        "Lint/FloatComparison"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_types(&self) -> &'static [u8] {
        &[CALL_NODE, FLOAT_NODE, INTEGER_NODE, NIL_NODE]
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

        let method = call.name().as_slice();
        let is_equality = matches!(method, b"==" | b"!=" | b"eql?" | b"equal?");
        if !is_equality {
            return;
        }

        let receiver = match call.receiver() {
            Some(r) => r,
            None => return,
        };

        let arguments = match call.arguments() {
            Some(a) => a,
            None => return,
        };

        let args = arguments.arguments();
        if args.len() != 1 {
            return;
        }

        let first_arg = args.iter().next().unwrap();

        // Skip safe comparisons: comparing to 0.0 or nil
        if is_literal_safe(&receiver) || is_literal_safe(&first_arg) {
            return;
        }

        if is_float(&receiver) || is_float(&first_arg) {
            let loc = call.location();
            let (line, column) = source.offset_to_line_col(loc.start_offset());
            let msg = if method == b"!=" {
                "Avoid inequality comparisons of floats as they are unreliable."
            } else {
                "Avoid equality comparisons of floats as they are unreliable."
            };
            diagnostics.push(self.diagnostic(source, line, column, msg.to_string()));
        }

    }
}

fn is_float(node: &ruby_prism::Node<'_>) -> bool {
    node.as_float_node().is_some()
}

fn is_literal_safe(node: &ruby_prism::Node<'_>) -> bool {
    // Comparing to 0.0 is safe
    if let Some(f) = node.as_float_node() {
        let src = f.location().as_slice();
        if src == b"0.0" || src == b"-0.0" {
            return true;
        }
    }
    // Comparing to integer 0 is safe
    if let Some(i) = node.as_integer_node() {
        let src = i.location().as_slice();
        if src == b"0" {
            return true;
        }
    }
    // Comparing to nil is safe
    if node.as_nil_node().is_some() {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FloatComparison, "cops/lint/float_comparison");
}
