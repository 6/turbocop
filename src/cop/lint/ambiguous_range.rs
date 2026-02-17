use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct AmbiguousRange;

impl Cop for AmbiguousRange {
    fn name(&self) -> &'static str {
        "Lint/AmbiguousRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let require_parens_for_chains =
            config.get_bool("RequireParenthesesForMethodChains", false);

        let range = match node.as_range_node() {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut diagnostics = Vec::new();

        // Check left boundary
        if let Some(left) = range.left() {
            if !is_acceptable_boundary(&left, require_parens_for_chains) {
                let loc = left.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Wrap complex range boundaries with parentheses to avoid ambiguity."
                        .to_string(),
                ));
            }
        }

        // Check right boundary
        if let Some(right) = range.right() {
            if !is_acceptable_boundary(&right, require_parens_for_chains) {
                let loc = right.location();
                let (line, column) = source.offset_to_line_col(loc.start_offset());
                diagnostics.push(self.diagnostic(
                    source,
                    line,
                    column,
                    "Wrap complex range boundaries with parentheses to avoid ambiguity."
                        .to_string(),
                ));
            }
        }

        diagnostics
    }
}

fn is_acceptable_boundary(node: &ruby_prism::Node<'_>, require_parens_for_chains: bool) -> bool {
    // Parenthesized expression
    if node.as_parentheses_node().is_some() {
        return true;
    }

    // Literals: integer, float, string, symbol, nil, true, false, rational, imaginary
    if node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
        || node.as_rational_node().is_some()
        || node.as_imaginary_node().is_some()
        || node.as_interpolated_string_node().is_some()
    {
        return true;
    }

    // Variables (local, instance, class, global)
    if node.as_local_variable_read_node().is_some()
        || node.as_instance_variable_read_node().is_some()
        || node.as_class_variable_read_node().is_some()
        || node.as_global_variable_read_node().is_some()
    {
        return true;
    }

    // Constants
    if node.as_constant_read_node().is_some() || node.as_constant_path_node().is_some() {
        return true;
    }

    // self
    if node.as_self_node().is_some() {
        return true;
    }

    // Method calls
    if let Some(call) = node.as_call_node() {
        // Unary operations (negation, etc) are acceptable
        let name = call.name().as_slice();
        if call.receiver().is_some()
            && call.arguments().is_none()
            && (name == b"-@" || name == b"+@" || name == b"~")
        {
            return true;
        }

        // Bare method calls (no receiver) are acceptable
        if call.receiver().is_none() {
            return true;
        }

        // Method calls on basic literals are NOT acceptable (e.g., 2.to_a in 1..2.to_a)
        if let Some(recv) = call.receiver() {
            if is_basic_literal(&recv) {
                return false;
            }
        }

        // Operator method calls (except []) are NOT acceptable
        // e.g., `x - 1` in `x - 1..2` or `x + 1` in `x + 1..2`
        if is_operator_method(name) && name != b"[]" {
            return false;
        }

        // Non-operator method calls with receiver: acceptable unless
        // RequireParenthesesForMethodChains is true
        return !require_parens_for_chains;
    }

    // OrNode, AndNode are NOT acceptable
    if node.as_or_node().is_some() || node.as_and_node().is_some() {
        return false;
    }

    false
}

fn is_operator_method(name: &[u8]) -> bool {
    matches!(
        name,
        b"|" | b"^"
            | b"&"
            | b"<=>"
            | b"=="
            | b"==="
            | b"=~"
            | b">"
            | b">="
            | b"<"
            | b"<="
            | b"<<"
            | b">>"
            | b"+"
            | b"-"
            | b"*"
            | b"/"
            | b"%"
            | b"**"
            | b"~"
            | b"+@"
            | b"-@"
            | b"!@"
            | b"~@"
            | b"[]"
            | b"[]="
            | b"!"
            | b"!="
            | b"!~"
            | b"`"
    )
}

fn is_basic_literal(node: &ruby_prism::Node<'_>) -> bool {
    node.as_integer_node().is_some()
        || node.as_float_node().is_some()
        || node.as_string_node().is_some()
        || node.as_symbol_node().is_some()
        || node.as_nil_node().is_some()
        || node.as_true_node().is_some()
        || node.as_false_node().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AmbiguousRange, "cops/lint/ambiguous_range");
}
