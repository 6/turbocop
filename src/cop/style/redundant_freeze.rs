use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct RedundantFreeze;

impl RedundantFreeze {
    fn is_immutable_literal(node: &ruby_prism::Node<'_>) -> bool {
        // Integers, floats, symbols, ranges, true, false, nil are immutable
        node.as_integer_node().is_some()
            || node.as_float_node().is_some()
            || node.as_rational_node().is_some()
            || node.as_imaginary_node().is_some()
            || node.as_symbol_node().is_some()
            || node.as_true_node().is_some()
            || node.as_false_node().is_some()
            || node.as_nil_node().is_some()
    }

    fn is_operation_producing_immutable(node: &ruby_prism::Node<'_>) -> bool {
        // Method calls on literals that return immutable values
        // e.g., 'foo'.count, [1,2,3].size, [1,2,3].count { ... }
        if let Some(call) = node.as_call_node() {
            let method = call.name();
            let name = method.as_slice();
            // Methods that return numeric/boolean values
            if matches!(
                name,
                b"count" | b"length" | b"size" | b"to_i" | b"to_f" | b"to_r" | b"to_c"
            ) {
                return true;
            }
            // Comparison operators produce booleans
            if matches!(
                name,
                b"<" | b">" | b"<=" | b">=" | b"==" | b"!=" | b"<=>"
            ) {
                return true;
            }
        }
        // Parenthesized expressions containing operations
        if let Some(parens) = node.as_parentheses_node() {
            if let Some(body) = parens.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().into_iter().collect();
                    if body_nodes.len() == 1 {
                        let inner = &body_nodes[0];
                        // Arithmetic or comparison => immutable result
                        if let Some(call) = inner.as_call_node() {
                            let method_name = call.name();
                            let name_bytes = method_name.as_slice();
                            if matches!(
                                name_bytes,
                                b"+" | b"-" | b"*" | b"/" | b"%" | b"**"
                                    | b"<" | b">" | b"<=" | b">=" | b"==" | b"!="
                            ) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }
}

impl Cop for RedundantFreeze {
    fn name(&self) -> &'static str {
        "Style/RedundantFreeze"
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: &ruby_prism::Node<'_>,
        _parse_result: &ruby_prism::ParseResult<'_>,
        _config: &CopConfig,
    ) -> Vec<Diagnostic> {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return Vec::new(),
        };

        // Must be a call to `.freeze` with no arguments
        if call_node.name().as_slice() != b"freeze" {
            return Vec::new();
        }
        if call_node.arguments().is_some() {
            return Vec::new();
        }

        // Must have a receiver
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Check if the receiver is an immutable literal
        let is_immutable = Self::is_immutable_literal(&receiver)
            || Self::is_operation_producing_immutable(&receiver);

        if !is_immutable {
            return Vec::new();
        }

        let loc = receiver.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        vec![self.diagnostic(
            source,
            line,
            column,
            "Do not freeze immutable objects, as freezing them has no effect.".to_string(),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantFreeze, "cops/style/redundant_freeze");
}
