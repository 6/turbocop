use crate::cop::node_type::{
    ARRAY_NODE, CALL_NODE, FALSE_NODE, FLOAT_NODE, IMAGINARY_NODE, INTEGER_NODE,
    INTERPOLATED_STRING_NODE, NIL_NODE, PARENTHESES_NODE, RATIONAL_NODE, STATEMENTS_NODE,
    STRING_NODE, SYMBOL_NODE, TRUE_NODE,
};
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

    fn is_numeric(node: &ruby_prism::Node<'_>) -> bool {
        node.as_integer_node().is_some() || node.as_float_node().is_some()
    }

    fn is_string_or_array(node: &ruby_prism::Node<'_>) -> bool {
        node.as_string_node().is_some()
            || node.as_interpolated_string_node().is_some()
            || node.as_array_node().is_some()
    }

    fn is_operation_producing_immutable(node: &ruby_prism::Node<'_>) -> bool {
        // Method calls that always return immutable values (integers).
        // count/length/size always return Integer regardless of receiver.
        if let Some(call) = node.as_call_node() {
            let method = call.name();
            let name = method.as_slice();
            if matches!(name, b"count" | b"length" | b"size") {
                return true;
            }
        }
        // Parenthesized expressions containing operations.
        // Must match the vendor's patterns precisely:
        //   (begin (send {float int} {:+ :- :* :** :/ :% :<<} _))
        //   (begin (send !{(str _) array} {:+ :- :* :** :/ :%} {float int}))
        //   (begin (send _ {:== :=== :!= :<= :>= :< :>} _))
        if let Some(parens) = node.as_parentheses_node() {
            if let Some(body) = parens.body() {
                if let Some(stmts) = body.as_statements_node() {
                    let body_nodes: Vec<_> = stmts.body().into_iter().collect();
                    if body_nodes.len() == 1 {
                        let inner = &body_nodes[0];
                        if let Some(call) = inner.as_call_node() {
                            let method_name = call.name();
                            let name_bytes = method_name.as_slice();

                            // Comparison operators always produce booleans (immutable)
                            if matches!(
                                name_bytes,
                                b"<" | b">" | b"<=" | b">=" | b"==" | b"===" | b"!="
                            ) {
                                return true;
                            }

                            // Arithmetic: only when operand types guarantee numeric result
                            let is_arithmetic = matches!(
                                name_bytes,
                                b"+" | b"-" | b"*" | b"/" | b"%" | b"**" | b"<<"
                            );
                            if is_arithmetic {
                                if let Some(receiver) = call.receiver() {
                                    // Pattern 1: numeric_left op anything
                                    if Self::is_numeric(&receiver) {
                                        return true;
                                    }
                                    // Pattern 2: non_string_non_array op numeric_right
                                    if !Self::is_string_or_array(&receiver) && name_bytes != b"<<" {
                                        if let Some(args) = call.arguments() {
                                            let arg_list: Vec<_> =
                                                args.arguments().iter().collect();
                                            if arg_list.len() == 1 && Self::is_numeric(&arg_list[0])
                                            {
                                                return true;
                                            }
                                        }
                                    }
                                }
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

    fn interested_node_types(&self) -> &'static [u8] {
        &[
            ARRAY_NODE,
            CALL_NODE,
            FALSE_NODE,
            FLOAT_NODE,
            IMAGINARY_NODE,
            INTEGER_NODE,
            INTERPOLATED_STRING_NODE,
            NIL_NODE,
            PARENTHESES_NODE,
            RATIONAL_NODE,
            STATEMENTS_NODE,
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let call_node = match node.as_call_node() {
            Some(c) => c,
            None => return,
        };

        // Must be a call to `.freeze` with no arguments
        if call_node.name().as_slice() != b"freeze" {
            return;
        }
        if call_node.arguments().is_some() {
            return;
        }

        // Must have a receiver
        let receiver = match call_node.receiver() {
            Some(r) => r,
            None => return,
        };

        // Check if the receiver is an immutable literal
        let is_immutable = Self::is_immutable_literal(&receiver)
            || Self::is_operation_producing_immutable(&receiver);

        if !is_immutable {
            return;
        }

        let loc = receiver.location();
        let (line, column) = source.offset_to_line_col(loc.start_offset());
        diagnostics.push(self.diagnostic(
            source,
            line,
            column,
            "Do not freeze immutable objects, as freezing them has no effect.".to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RedundantFreeze, "cops/style/redundant_freeze");
}
